// root node:
// 1) url - key
// 2) list of variance headers. N.B: how to deal with auth headers?
// 3) number of data nodes (to remove the root node once no data nodes are present)
//
// data node:
// 1) list of variance header names
// 2) expiration info
// 3) guid to lru cache

mod internal_expiration_queue;
mod internal_lru_cache;
mod internal_uri_map;
mod memory_cached_response_body;

use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Display,
    future::{Future, ready},
    ops::DerefMut,
    pin,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use bytes::Bytes;
// use memory_cached_response_body::MemoryCachedResponse;
use rama_core::error::BoxError;
use rama_http_types::dep::http_body::{Body, Frame};
use rama_http_types::{
    Uri,
    dep::http::{Response, request, response},
};
use tokio_stream::StreamExt;

use crate::layer::util::{
    body_stream_wrapper,
    intercept_body::{InterceptedBody, intercept_body},
};

use super::{byte_body::ByteBody, CacheKey, CacheStorage};

pub(crate) struct MemoryCacheStorage {
    internal: Arc<Mutex<MemoryCacheStorageInternal>>,
}

struct MemoryCacheStorageInternal {
    uri_map: internal_uri_map::UriMap,
    lru_cache:
        internal_lru_cache::LruStorage<(response::Parts, Vec<Frame<Bytes>>, Vec<(String, String)>)>,
    expiration_queue: internal_expiration_queue::ExpirationQueue,
    in_flight_requests: HashSet<CacheKey>,
}

// TODO: we are cloning the version here - is it correct?
impl CacheStorage for MemoryCacheStorage {
    type CachedResponseBody = ByteBody;

    type InterceptedResponseBody<InnerBody: Body + Send + 'static>
        = InterceptedBody<InnerBody>
    where
        InnerBody::Error: Into<BoxError>;

    fn get_response(
        &self,
        request_head: &request::Parts,
    ) -> impl Future<
        Output = Result<Option<(Response<Self::CachedResponseBody>, Vec<(String, String)>)>, BoxError>,
    > + Send
    + 'static {
        let mut internal = self.internal.lock().unwrap();

        let cache_key = CacheKey::from_req_variance(
            request_head,
            internal
                .uri_map
                .get_variance(&request_head.uri)
                .map(|header_keys| header_keys.into_iter().map(|header_key| header_key.clone())),
        );

        let Some(expiration) = internal.expiration_queue.get_expiration(&cache_key) else {
            return ready(Ok(None));
        };

        if *expiration < SystemTime::now() {
            let Some(data) = internal.lru_cache.peek_item(&cache_key) else {
                return ready(Ok(None));
            };

            let Some(cached_body) = ByteBody::from_byte_vec(&data.1) else {
                return ready(Ok(None));
            };

            let cached_response = Response::from_parts(data.0.clone(), cached_body);

            let metadata = data.2.iter()
                .map(|t| (t.0.clone(), t.1.clone()))
                .collect();

            internal.lru_cache.promote_item(&cache_key);
            return ready(Ok(Some((cached_response, metadata))));
        }

        internal.lru_cache.remove_item(&cache_key);
        internal.expiration_queue.remove_item(&cache_key);
        // TODO: why are we sending uri separately here, if cache_key already has one?
        internal.uri_map.remove_key(&request_head.uri, &cache_key);

        return ready(Ok(None));
    }

    fn set_response<OriginalBody: Body + Send + 'static>(
        &self,
        key: Cow<'_, CacheKey>,
        expiration: SystemTime,
        metadata: &[(&str, &str)],
        response: Response<OriginalBody>,
    ) -> impl Future<
        Output = Result<
            Response<Self::InterceptedResponseBody<OriginalBody>>,
            (Response<OriginalBody>, BoxError),
        >,
    > + Send
    + 'static
    where
        OriginalBody::Error: Into<BoxError>,
    {
        let (headers, original_body) = response.into_parts();

        let mut state_guard = self.internal.lock().unwrap();
        if state_guard.in_flight_requests.contains(key.as_ref()) {
            return ready(Err((
                Response::from_parts(headers, original_body),
                Box::new(CacheError {
                    message: "Failed to cache, because this key is already being cached".to_owned(),
                }) as BoxError,
            )));
        }

        let interception_result = intercept_body(original_body);

        let (intercepted, intercepting) = (
            interception_result.intercepted,
            interception_result.intercepting,
        );

        let key = key.into_owned();
        state_guard.in_flight_requests.insert(key.clone());
        drop(state_guard);

        let state_ref = Arc::clone(&self.internal);
        let headers_clone = headers.clone();

        let metadata = metadata
            .into_iter()
            .map(|t| (t.0.to_owned(), t.1.to_owned()))
            .collect();

        tokio::spawn(async move {
            let mut frames: Vec<Frame<Bytes>> = Vec::new();

            let mut stream_wrapper =
                pin::pin!(body_stream_wrapper::BodySreamWrapper::new(intercepting));

            while let Some(next_result) = stream_wrapper.next().await {
                match next_result {
                    Ok(frame) => frames.push(frame),
                    Err(_) => {
                        let mut state_guard = state_ref.lock().unwrap();
                        state_guard.in_flight_requests.remove(&key);
                        return;
                    }
                }
            }

            let mut state_guard = state_ref.lock().unwrap();

            state_guard.in_flight_requests.remove(&key);
            if let Some(removed_keys) = state_guard
                .uri_map
                .add_key(Cow::Owned(key.get_uri().clone()), &key)
            {
                for key in removed_keys {
                    state_guard.lru_cache.remove_item(&key);
                    state_guard.expiration_queue.remove_item(&key);
                }
            }

            let frames_size = frames
                .iter()
                .fold(0, |total_size: usize, frame: &Frame<Bytes>| {
                    total_size + frame.data_ref().map_or(0, |data_ref| data_ref.len())
                });
            state_guard
                .expiration_queue
                .set_expiration(key.clone(), expiration);
            state_guard
                .lru_cache
                .set_item(key, (headers_clone, frames, metadata), frames_size);

            if state_guard.lru_cache.is_size_exceeded() {
                MemoryCacheStorage::clear_expired_items(state_guard.deref_mut());
            }

            if state_guard.lru_cache.is_size_exceeded() {
                MemoryCacheStorage::clear_least_used_items(state_guard.deref_mut());
            }
        });

        return ready(Ok(Response::from_parts(headers, intercepted)));
    }

    fn invalidate_url(
        &self,
        uri: &Uri,
    ) -> impl Future<Output = Result<(), BoxError>> + Send + 'static {
        let mut guard = self.internal.lock().unwrap();
        let internal_state = guard.deref_mut();
        let Some(removed_keys) = internal_state.uri_map.remove(uri) else {
            return ready(Ok(()));
        };

        for key in removed_keys {
            internal_state.lru_cache.remove_item(&key);
            internal_state.expiration_queue.remove_item(&key);
        }

        return ready(Ok(()));
    }
}

impl MemoryCacheStorage {
    fn clear_expired_items(internal_state: &mut MemoryCacheStorageInternal) {
        let now = SystemTime::now();

        loop {
            let Some((cache_key, expiration_time)) =
                internal_state.expiration_queue.peek_first_to_expire()
            else {
                return;
            };

            let Ok(_) = now.duration_since(expiration_time.clone()) else {
                return;
            };

            let cache_key = cache_key.clone();
            internal_state.expiration_queue.remove_item(&cache_key);
            internal_state.lru_cache.remove_item(&cache_key);
            internal_state
                .uri_map
                .remove_key(cache_key.get_uri(), &cache_key);
        }
    }

    fn clear_least_used_items(internal_state: &mut MemoryCacheStorageInternal) {
        while internal_state.lru_cache.is_size_exceeded() {
            let Some(evicted) = internal_state.lru_cache.evict_one() else {
                return;
            };

            internal_state.expiration_queue.remove_item(&evicted);
            internal_state
                .uri_map
                .remove_key(evicted.get_uri(), &evicted);
        }
    }
}

#[derive(Debug, Clone)]
struct CacheError {
    message: String,
}

impl Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return self.message.fmt(f);
    }
}

impl std::error::Error for CacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        return "Cache Error. Please, use the Display trait for more details";
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

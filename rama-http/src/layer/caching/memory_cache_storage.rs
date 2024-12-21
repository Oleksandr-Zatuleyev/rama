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
    future::{ready, Future},
    pin,
    sync::{Arc, Mutex},
    time::Instant,
};

use bytes::Bytes;
use http::{request, response, Response};
use http_body::{Body, Frame};
use memory_cached_response_body::MemoryCachedResponse;
use rama_core::error::BoxError;
use tokio_stream::StreamExt;

use crate::layer::util::{
    body_stream_wrapper,
    intercept_body::{intercept_body, InterceptedBody},
    union_body::UnionBody,
};

use super::{CacheKey, CacheStorage};

pub(crate) struct MemoryCacheStorage {
    internal: Arc<Mutex<MemoryCacheStorageInternal>>,
}

struct MemoryCacheStorageInternal {
    uri_map: internal_uri_map::UriMap,
    lru_cache: internal_lru_cache::LruStorage<(response::Parts, Vec<Frame<Bytes>>)>,
    expiration_queue: internal_expiration_queue::ExpirationQueue,
    in_flight_requests: HashSet<CacheKey>,
}

impl CacheStorage for MemoryCacheStorage {
    type CachedResponseBody = memory_cached_response_body::MemoryCachedResponse;

    type InterceptedResponseBody<InnerBody: Body + Send + 'static> = UnionBody<InnerBody, InterceptedBody<InnerBody>>
    where
        InnerBody::Error: Into<BoxError>;

    fn get_response(
        &self,
        request_head: &request::Parts,
    ) -> impl Future<Output = Result<Option<Response<Self::CachedResponseBody>>, BoxError>>
           + Send
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

        if *expiration < Instant::now() {
            let Some(data) = internal.lru_cache.peek_item(&cache_key) else {
                return ready(Ok(None));
            };

            let Some(cached_body) = MemoryCachedResponse::from_byte_vec(&data.1) else {
                return ready(Ok(None));
            };

            let cached_response = Response::from_parts(data.0.clone(), cached_body);

            internal.lru_cache.promote_item(&cache_key);
            return ready(Ok(Some(cached_response)));
        }

        internal.lru_cache.remove_item(&cache_key);
        internal.expiration_queue.remove_item(&cache_key);
        // TODO: why are we sending uri separately here, if cache_key already has one?
        internal.uri_map.remove_key(&request_head.uri, &cache_key);

        return ready(Ok(None));
    }

    /// TODO
    fn set_response<OriginalBody: Body + Send + 'static>(
        &self,
        key: Cow<'_, CacheKey>,
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
            return ready(Ok(Response::from_parts(
                headers,
                UnionBody::first(original_body),
            )));
        }

        let interception_result = intercept_body(original_body);

        let (intercepted, intercepting) = (
            interception_result.intercepted,
            interception_result.intercepting,
        );

        let key = key.into_owned();
        state_guard.in_flight_requests.insert(key.clone());

        let state_ref = Arc::clone(&self.internal);
        let cache_headers = headers.clone();
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
            if let Some(removed_keys) = state_guard.uri_map.add_key(Cow::Owned(key.get_uri().clone()), key) {
                for key in removed_keys {
                    state_guard.lru_cache.remove_item(&key);
                    state_guard.expiration_queue.remove_item(&key);
                }
            }

            state_guard.expiration_queue.set_expiration(key.clone(), expiration);
            // TODO: add to expiration queue
            // TODO: add to lru cache
        });

        return ready(Ok(Response::from_parts(
            headers,
            UnionBody::second(intercepted),
        )));
    }

    fn invalidate_response(
        &self,
        uri: &http::Uri,
    ) -> impl Future<Output = Result<(), BoxError>> + Send + 'static {
        async {
            todo!();
        }
    }
}

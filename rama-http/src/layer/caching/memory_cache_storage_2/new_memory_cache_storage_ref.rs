use std::{
    collections::HashMap,
    pin,
    sync::Arc,
    time::{Duration, SystemTime},
};

use rama_core::{
    context::Extensions, error::BoxError, telemetry::opentelemetry::propagation::Injector,
};
use rama_http_types::{HeaderMap, StatusCode};
use tokio_stream::StreamExt;

use crate::layer::{
    caching::{CacheKey, CacheRef, NewCacheRef},
    util::{
        body_stream_wrapper,
        intercept_body::{InterceptedBody, intercept_body},
    },
};

use super::memory_cache_storage_repository::{AddItemParams, MemoryCacheStorageRepository};

pub struct NewMemoryCacheStorageRef {
    cache_key: CacheKey,
    repository: Arc<MemoryCacheStorageRepository>,
    status_code: StatusCode,
    expiration: SystemTime,
    metadata: HashMap<String, String>,
    headers: HeaderMap,
}

impl NewMemoryCacheStorageRef {
    pub(super) fn new(
        cache_key: CacheKey,
        repository: Arc<MemoryCacheStorageRepository>,
    ) -> NewMemoryCacheStorageRef {
        return NewMemoryCacheStorageRef {
            cache_key,
            repository,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            expiration: SystemTime::now()
                .checked_sub(Duration::from_secs(1))
                .unwrap(),
            metadata: HashMap::new(),
            headers: HeaderMap::new(),
        };
    }
}

impl CacheRef for NewMemoryCacheStorageRef {
    fn get_cache_key(&self) -> &CacheKey {
        return &self.cache_key;
    }

    fn get_status(&self) -> StatusCode {
        return self.status_code;
    }

    fn get_expiration(&self) -> SystemTime {
        return self.expiration;
    }

    fn set_expiration(&mut self, time: SystemTime) {
        self.expiration = time;
    }

    fn get_metadata(&self, key: &str) -> Option<&str> {
        return self.metadata.get(key).map(|val| val.as_str());
    }

    fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.set(key.as_str(), value);
    }

    fn get_headers(&self) -> &HeaderMap {
        return &self.headers;
    }

    fn get_headers_mut(&mut self) -> &mut HeaderMap {
        return &mut self.headers;
    }
}

impl NewCacheRef for NewMemoryCacheStorageRef {
    type InterceptedResponseBody<
        InnerBody: rama_http_types::dep::http_body::Body<Error: Into<rama_core::error::BoxError>>
            + Send
            + 'static,
    > = InterceptedBody<InnerBody>;

    fn set_status(&mut self, status: StatusCode) {
        self.status_code = status;
    }

    fn commit_response<
        OriginalBody: rama_http_types::dep::http_body::Body<Error: Into<rama_core::error::BoxError>>
            + Send
            + 'static,
    >(
        self,
        original_body: OriginalBody,
    ) -> impl Future<
        Output = Result<
            Self::InterceptedResponseBody<OriginalBody>,
            (OriginalBody, impl Into<rama_core::error::BoxError>),
        >,
    > + Send {
        async move {
            let interception_result = intercept_body(original_body);

            let intercepting = interception_result.intercepting;

            tokio::spawn(async move {
                let mut byte_frames = Vec::new();
                let mut trailers = None;

                let mut stream_wrapper =
                    pin::pin!(body_stream_wrapper::BodySreamWrapper::new(intercepting));

                while let Some(next_result) = stream_wrapper.next().await {
                    let Ok(frame) = next_result else {
                        return;
                    };

                    let frame = match frame.into_data() {
                        Ok(bytes) => {
                            byte_frames.push(bytes);
                            continue;
                        }
                        Err(frame) => frame,
                    };

                    match frame.into_trailers() {
                        Ok(trailer_frame) => {
                            let Some(ref mut existing_trailers) = trailers else {
                                trailers = Some(trailer_frame);
                                continue;
                            };

                            for (header_name, header_value) in trailer_frame.into_iter() {
                                let Some(header_name) = header_name else {
                                    continue;
                                };

                                existing_trailers.append(header_name, header_value);
                            }
                        }
                        Err(_) => {}
                    };
                }

                let content_length: usize = byte_frames.iter().map(|b| b.len()).sum();
                let Ok(content_length) = content_length.try_into() else {
                    return;
                };

                self.repository
                    .add_item(AddItemParams {
                        cache_key: self.cache_key,
                        body: byte_frames,
                        status: self.status_code,
                        headers: self.headers,
                        extensions: Extensions::default(),
                        trailers: trailers.unwrap_or_default(),
                        metadata: self.metadata,
                        expiration: self.expiration,
                        content_length: content_length,
                    })
                    .await;
            });

            return Result::<_, (OriginalBody, BoxError)>::Ok(interception_result.intercepted);
        }
    }
}

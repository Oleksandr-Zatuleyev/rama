use std::{collections::HashMap, future, ops::Deref, sync::Arc, time::SystemTime};

use rama_core::error::BoxError;
use rama_http_types::HeaderMap;

use crate::layer::caching::{CacheRef, ExistingCacheRef, byte_body::ByteBody};

use super::{MemoryCacheItem, memory_cache_storage_repository::MemoryCacheStorageRepository};

pub struct ExistingMemoryCacheStorageRef {
    repository: Arc<MemoryCacheStorageRepository>,
    cache_item: Arc<MemoryCacheItem>,

    // TODO: can we do it better so that this whole thing is not cloned every time?
    expiration: SystemTime,
    metadata: HashMap<String, String>,
    headers: HeaderMap,
    trailers: HeaderMap,
}

impl ExistingMemoryCacheStorageRef {
    pub(super) fn new(
        repository: Arc<MemoryCacheStorageRepository>,
        cache_item: Arc<MemoryCacheItem>,
    ) -> ExistingMemoryCacheStorageRef {
        let read_guard = cache_item.mutable_state.read().unwrap();
        let expiration = read_guard.expiration.clone();
        let metadata = read_guard.metadata.clone();
        let headers = read_guard.headers.clone();
        let trailers = read_guard.trailers.clone();
        drop(read_guard);

        return ExistingMemoryCacheStorageRef {
            repository: repository,
            cache_item: cache_item,
            expiration,
            metadata,
            headers,
            trailers,
        };
    }
}

impl CacheRef for ExistingMemoryCacheStorageRef {
    fn get_cache_key(&self) -> &crate::layer::caching::CacheKey {
        return &self.cache_item.deref().cache_key;
    }

    fn get_expiration(&self) -> SystemTime {
        return self.expiration.clone();
    }

    fn set_expiration(&mut self, time: SystemTime) {
        self.expiration = time;
    }

    fn get_metadata(&self, key: &str) -> Option<&str> {
        return self.metadata.get(key).map(|s| s.as_str());
    }

    fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    fn get_headers(&self) -> &HeaderMap {
        return &self.headers;
    }

    fn get_headers_mut(&mut self) -> &mut HeaderMap {
        return &mut self.headers;
    }

    fn get_status(&self) -> rama_http_types::StatusCode {
        return self.cache_item.status;
    }
}

impl ExistingCacheRef for ExistingMemoryCacheStorageRef {
    type CachedResponseBody = ByteBody;

    fn invalidate(self) -> impl Future<Output = Result<(), impl Into<BoxError>>> + Send {
        self.repository
            .invalidate_item(&self.cache_item.cache_key, self.cache_item.body_id);

        return future::ready(Result::<(), BoxError>::Ok(()));
    }

    // TODO: concurrency conflict resolution strategies?
    fn commit_changes(
        &mut self,
    ) -> impl Future<Output = Result<(), impl Into<BoxError>>> + Send + '_ {
        async {
            {
                let mut write_guard = self.cache_item.mutable_state.write().unwrap();

                // TODO: update expiration queue if expiration changes
                write_guard.expiration = self.expiration;

                write_guard.metadata = self.metadata.clone();
                write_guard.headers = self.headers.clone();
                write_guard.trailers = self.trailers.clone();
            }

            self.repository.notify_cache_item_updated(&self.cache_item).await;

            return Result::<_, BoxError>::Ok(());
        }
    }

    fn get_response_body(
        &self,
    ) -> impl Future<Output = Result<Self::CachedResponseBody, impl Into<BoxError>>> + Send + '_
    {
        let body = &self.cache_item.body;
        let trailers = self
            .cache_item
            .mutable_state
            .read()
            .unwrap()
            .trailers
            .clone();

        let body = ByteBody::from_bytes_and_trailers(body, Some(trailers));

        return future::ready(Result::<_, BoxError>::Ok(body));
    }

    fn get_trailers(&self) -> &HeaderMap {
        return &self.headers;
    }

    fn get_trailers_mut(&mut self) -> &mut HeaderMap {
        return &mut self.headers;
    }
}

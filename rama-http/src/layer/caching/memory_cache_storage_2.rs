mod existing_memory_cache_storage_ref;
mod memory_cache_item;
mod memory_cache_storage_repository;
mod new_memory_cache_storage_ref;

use std::future;
use std::sync::Arc;

use existing_memory_cache_storage_ref::ExistingMemoryCacheStorageRef;
use memory_cache_item::MemoryCacheItem;
use memory_cache_storage_repository::MemoryCacheStorageRepository;
use new_memory_cache_storage_ref::NewMemoryCacheStorageRef;
use rama_core::error::{BoxError, error};
use rama_http_types::HeaderMap;

use crate::layer::util::intercept_body::InterceptedBody;

use super::byte_body::ByteBody;
use super::{CacheKey, CacheStorage2};

pub struct MemoryCacheStorage2 {
    repository: Arc<MemoryCacheStorageRepository>,
}

impl MemoryCacheStorage2 {
    pub fn new(max_size: usize) -> MemoryCacheStorage2 {
        return MemoryCacheStorage2 {
            repository: MemoryCacheStorageRepository::new(max_size),
        };
    }
}

impl CacheStorage2 for MemoryCacheStorage2 {
    type NewItem<
        InnerBody: rama_http_types::dep::http_body::Body<Error: Into<rama_core::error::BoxError>>
            + Send
            + 'static,
    > = NewMemoryCacheStorageRef;

    type ExistingItem = ExistingMemoryCacheStorageRef;

    type CachedResponseBody = ByteBody;

    type InterceptedResponseBody<
        InnerBody: rama_http_types::dep::http_body::Body<Error: Into<rama_core::error::BoxError>>
            + Send
            + 'static,
    > = InterceptedBody<InnerBody>;

    fn get_item(
        &self,
        uri: &rama_http_types::Uri,
        request_headers: &HeaderMap,
    ) -> impl Future<Output = Result<Self::ExistingItem, impl Into<rama_core::error::BoxError>>>
    + Send
    + '_ {
        let Some(cache_item) = self.repository.get_item(uri, request_headers) else {
            return future::ready(Err(Box::new(error!("not found"))));
        };

        return future::ready(Ok(ExistingMemoryCacheStorageRef::new(
            self.repository.clone(),
            cache_item
        )));
    }

    fn add_item<OriginalBody: rama_http_types::dep::http_body::Body + Send + 'static>(
        &self,
        key: std::borrow::Cow<'_, CacheKey>,
        _estimated_size: usize,
    ) -> impl Future<
        Output = Result<Self::NewItem<OriginalBody>, impl Into<rama_core::error::BoxError>>,
    > + Send
    + '_
    where
        OriginalBody::Data: Send + 'static,
        OriginalBody::Error: Into<rama_core::error::BoxError>,
    {
        return future::ready(Result::<_, BoxError>::Ok(NewMemoryCacheStorageRef::new(
            key.into_owned(),
            self.repository.clone(),
        )));
    }

    fn invalidate_url(
        &self,
        uri: &rama_http_types::Uri,
    ) -> impl Future<Output = Result<(), impl Into<rama_core::error::BoxError>>> + Send + '_ {
        self.repository.invalidate_url(uri);

        return future::ready(Result::<_, BoxError>::Ok(()));
    }
}

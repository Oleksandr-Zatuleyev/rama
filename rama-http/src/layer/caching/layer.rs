use rama_core::Layer;

use super::{
    CacheStorage, CachingService, DefaultCachePurgePredicate, DefaultCacheReadPredicate,
    DefaultCacheStorePredicate,
};

/// A layer that implements caching
#[derive(Debug)]
pub struct CachingLayer<Storage: CacheStorage + Send + Sync + 'static, StorageFactory: Fn() -> Storage> {
    storage_factory: StorageFactory,
}

impl<Storage: CacheStorage + Send + Sync + 'static, StorageFactory: Fn() -> Storage> CachingLayer<Storage, StorageFactory> {
    /// Creates a new caching layer
    pub fn new(storage_factory: StorageFactory) -> CachingLayer<Storage, StorageFactory> {
        return CachingLayer { storage_factory };
    }
}

// TODO: check https://users.rust-lang.org/t/unconstrained-type-parameter-when-the-parameter-is-constrained-by-a-where-clause/95512/7 on how to resolve this
impl<InnerService, Storage: CacheStorage + Send + Sync + 'static, StorageFactory: Fn() -> Storage>
    Layer<InnerService> for CachingLayer<Storage, StorageFactory>
{
    type Service = CachingService<
        InnerService,
        DefaultCacheReadPredicate,
        DefaultCacheStorePredicate,
        DefaultCachePurgePredicate,
        Storage
    >;

    fn layer(&self, inner: InnerService) -> Self::Service {
        return CachingService::new(
            inner,
            DefaultCacheReadPredicate,
            DefaultCacheStorePredicate,
            DefaultCachePurgePredicate,
            (self.storage_factory)(),
        );
    }
}

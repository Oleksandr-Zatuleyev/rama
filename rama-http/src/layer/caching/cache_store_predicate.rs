use http::{request::Parts as RequestParts, response::Parts as ResponseParts};

/// Checks if the response should be stored in cache
pub trait CacheStorePredicate {
    /// Returns true when the response should be stored in cache
    fn should_store(&self, request_head: &RequestParts, response_head: &ResponseParts) -> bool;
}

/// The default implementation of CacheStorePredicate
#[derive(Debug)]
pub struct DefaultCacheStorePredicate;

impl CacheStorePredicate for DefaultCacheStorePredicate {
    fn should_store(&self, request_head: &RequestParts, response_head: &ResponseParts) -> bool {
        return true;
    }
}
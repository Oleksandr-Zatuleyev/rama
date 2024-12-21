use http::request::Parts;

/// Decides when the request should purge cache items associated with the url of the request
pub trait CachePurgePredicate {
    /// Returns true when cache items should be purged
    fn should_purge(&self, request_head: &Parts) -> bool;
}

/// The default implementation of cache purge predicate
#[derive(Debug)]
pub struct DefaultCachePurgePredicate;

impl CachePurgePredicate for DefaultCachePurgePredicate {
    fn should_purge(&self, request_head: &Parts) -> bool {
        return !request_head.method.is_safe();
    }
}

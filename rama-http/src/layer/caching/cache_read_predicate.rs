use http::{request::Parts, Method};

/// Checks if the request could be served from cache
pub trait CacheReadPredicate {
    /// Returns true if the request could be served from cache
    fn can_read_from_cache(&self, request_head: &Parts) -> bool;
}

/// The default implementation for CacheReadPredicate
#[derive(Debug)]
pub struct DefaultCacheReadPredicate;

impl CacheReadPredicate for DefaultCacheReadPredicate {
    fn can_read_from_cache(&self, request_head: &Parts) -> bool {
        if !can_serve_method_from_cache(&request_head.method) || is_range_request(request_head) {
            return false;
        }

        return true;
    }
}

fn can_serve_method_from_cache(method: &Method) -> bool {
    // TODO: figure out how to deal with HEAD requests
    return method == Method::GET;
}

fn is_range_request(request_head: &Parts) -> bool {
    // TODO: range requests not supported for the time being
    return request_head.headers.contains_key("Range");
}

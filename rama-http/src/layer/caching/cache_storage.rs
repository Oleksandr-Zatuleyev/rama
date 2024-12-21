use std::{borrow::Cow, future::Future};

use http::{request::Parts, Uri};
use http_body::Body;
use rama_core::error::BoxError;
use rama_http_types::Response;

use super::cache_key::CacheKey;

/// need to:
/// 1) get response by request head
/// 2) store response by key
/// 3) invalidate response by uri
/// 4) invalidate resopnse by uri that does not meet some optional criteria: too old, does not match etag etc - ???
/// TODO
pub trait CacheStorage {
    /// TODO
    type CachedResponseBody: Body<Error: Into<BoxError>> + Send + 'static;
    /// TODO
    type InterceptedResponseBody<InnerBody: Body + Send + 'static>: Body<Error: Into<BoxError>>
        + Send
        + 'static
    where
        InnerBody::Error: Into<BoxError>;

    /// TODO
    fn get_response(
        &self,
        request_head: &Parts,
    ) -> impl Future<Output = Result<Option<Response<Self::CachedResponseBody>>, BoxError>>
           + Send
           + 'static;

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
        OriginalBody::Data: Send + 'static,
        OriginalBody::Error: Into<BoxError>;

    /// TODO
    fn invalidate_response(
        &self,
        uri: &Uri,
    ) -> impl Future<Output = Result<(), BoxError>> + Send + 'static;

    // TODO: additional methods to support request collapsing, getting only headers?
}

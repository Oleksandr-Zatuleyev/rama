use std::{borrow::Cow, time::SystemTime};

use rama_core::error::BoxError;
use rama_http_types::{
    dep::http_body::Body, HeaderMap, StatusCode, Uri
};

use super::CacheKey;

pub trait CacheStorage2 {
    type NewItem<InnerBody: Body<Error: Into<BoxError>> + Send + 'static>: NewCacheRef<
        InterceptedResponseBody<InnerBody> = Self::InterceptedResponseBody<InnerBody>,
    >;
    type ExistingItem: ExistingCacheRef<CachedResponseBody = Self::CachedResponseBody>;
    type CachedResponseBody: Body<Error: Into<BoxError>> + Send + 'static;
    type InterceptedResponseBody<InnerBody: Body<Error: Into<BoxError>> + Send + 'static>: Body<Error: Into<BoxError>>
        + Send
        + 'static;

    fn get_item(
        &self,
        uri: &Uri,
        request_headers: &HeaderMap,
    ) -> impl Future<Output = Result<Self::ExistingItem, impl Into<BoxError>>> + Send + '_;

    fn add_item<OriginalBody: Body + Send + 'static>(
        &self,
        key: Cow<'_, CacheKey>,
        estimated_size: usize,
    ) -> impl Future<Output = Result<Self::NewItem<OriginalBody>, impl Into<BoxError>>> + Send + '_
    where
        OriginalBody::Data: Send + 'static,
        OriginalBody::Error: Into<BoxError>;

    fn invalidate_url(
        &self,
        uri: &Uri,
    ) -> impl Future<Output = Result<(), impl Into<BoxError>>> + Send + '_;
}

// TODO: think of efficiency implications regarding always returning the whole header map
pub trait CacheRef: Send + 'static {
    fn get_cache_key(&self) -> &CacheKey;

    fn get_status(&self) -> StatusCode;

    fn get_expiration(&self) -> SystemTime;

    fn set_expiration(&mut self, time: SystemTime);

    fn get_metadata(&self, key: &str) -> Option<&str>;

    fn set_metadata(&mut self, key: String, value: String);

    fn get_headers(&self) -> &HeaderMap;

    fn get_headers_mut(&mut self) -> &mut HeaderMap;
}

pub trait NewCacheRef: CacheRef {
    type InterceptedResponseBody<InnerBody: Body<Error: Into<BoxError>> + Send + 'static>: Body<Error: Into<BoxError>>
        + Send
        + 'static;

    fn set_status(&mut self, status: StatusCode);

    fn commit_response<OriginalBody: Body<Error: Into<BoxError>> + Send + 'static>(
        self,
        original_body: OriginalBody,
    ) -> impl Future<
        Output = Result<
            Self::InterceptedResponseBody<OriginalBody>,
            (OriginalBody, impl Into<BoxError>),
        >,
    > + Send;
}

pub trait ExistingCacheRef: CacheRef {
    type CachedResponseBody: Body<Error: Into<BoxError>> + Send;

    fn invalidate(self) -> impl Future<Output = Result<(), impl Into<BoxError>>> + Send;

    fn commit_changes(&mut self) -> impl Future<Output = Result<(), impl Into<BoxError>>> + Send + '_;

    fn get_response_body(
        &self,
    ) -> impl Future<Output = Result<Self::CachedResponseBody, impl Into<BoxError>>> + Send + '_;

    fn get_trailers(&self) -> &HeaderMap;

    fn get_trailers_mut(&mut self) -> &mut HeaderMap;
}

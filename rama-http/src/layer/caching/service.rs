use std::str::FromStr;
use std::{borrow::Cow, fmt::Debug, time::SystemTime};

use bytes::Buf;
use chrono::{DateTime, FixedOffset, Utc};
use rama_core::{Context, Service, error::BoxError};
use rama_http_types::dep::http::{self, Method, Request, Response, StatusCode, request, response};
use rama_http_types::dep::http_body::Body;
use rama_http_types::header::Entry;
use rama_http_types::headers::{
    CacheControl, Date, ETag, Header, IfModifiedSince, IfNoneMatch, LastModified,
};
use rama_http_types::{HeaderMap, HeaderName, HeaderValue};

use crate::layer::util::union_body::{UnionBody, UnionBodyVariant};

use super::byte_body::ByteBody;
use super::caching_utils::{GetAgeParams, get_current_age};
use super::{CacheStorage, cache_key::CacheKey, caching_utils::get_expiration_time};

static NON_STORED_HEADERS: &[&str] = &[
    "Connection",
    "Proxy-Connection",
    "Keep-Alive",
    "TE",
    "Transfer-Encoding",
    "Upgrade",
    "Proxy-Authenticate",
    "Proxy-Authentication-Info",
    "Proxy-Authorization",
];

static NOT_MODIFIED_RESPONSE_HEADERS: &[&str] = &[
    "Content-Location",
    "Date",
    "ETag",
    "Vary",
    "Cache-Control",
    "Expires",
    "Last-Modified",
];

static RESPONSE_DATE_KEY: &str = "RESPONSE_DATE";
static REQUEST_DATE_KEY: &str = "REQUEST_DATE";

// UnionBody3<
// ResponseBody,
// Storage::CachedResponseBody,
// Storage::InterceptedResponseBody<ResponseBody>,
// >,
type CacheResponseBodyVariant<OriginalResponseBody, CachedResponseBody, InterceptedResponseBody> =
    UnionBody<
        UnionBody<OriginalResponseBody, CachedResponseBody>,
        UnionBody<InterceptedResponseBody, ByteBody>,
    >;

impl<
    OriginalResponseBody: Body<Error: Into<BoxError>>,
    CachedResponseBody: Body<Error: Into<BoxError>>,
    InterceptedResponseBody: Body<Error: Into<BoxError>>,
> CacheResponseBodyVariant<OriginalResponseBody, CachedResponseBody, InterceptedResponseBody>
{
    fn new_original_response(
        body: OriginalResponseBody,
    ) -> CacheResponseBodyVariant<OriginalResponseBody, CachedResponseBody, InterceptedResponseBody>
    {
        return UnionBody::first(UnionBody::first(body));
    }

    fn new_cached_response(
        body: CachedResponseBody,
    ) -> CacheResponseBodyVariant<OriginalResponseBody, CachedResponseBody, InterceptedResponseBody>
    {
        return UnionBody::first(UnionBody::second(body));
    }

    fn new_intercepted_response(
        body: InterceptedResponseBody,
    ) -> CacheResponseBodyVariant<OriginalResponseBody, CachedResponseBody, InterceptedResponseBody>
    {
        return UnionBody::second(UnionBody::first(body));
    }

    fn new_byte_response(
        body: ByteBody,
    ) -> CacheResponseBodyVariant<OriginalResponseBody, CachedResponseBody, InterceptedResponseBody>
    {
        return UnionBody::second(UnionBody::second(body));
    }
}

/*
Rules
    if no-store or no-cache - skip caching - Done
    if private cache directive is present - skip - Done
    if the request has "Authorize" header and there is no "public" directive and no "s-maxage" directive and no "must-revalidate" directive - skip - Done
    do not store response if the response contains "private" directive - done
    skip, if none of these are true:
        public response directive,
        Expires header field is available
        maxage is available
        s-maxage is available

        on second thought, this should not be necessary, because there is another condition added via "or":
            (in section 3)
            a status code that is defined as heuristically cacheable (see Section 4.2.2).
            basically TODO later - caching responses if the status code is not "heuristically cacheable", but caching is still allowed by some of the 4 conditions above

    do not store headers: - done, these headers are removed when stored, but caching is not responsible for removing them before forwarding
        "Connection" header and all headers listed in "Connection" header
        Proxy-Connection, Keep-Alive, TE, Transfer-Encoding, Upgrade
        Proxy-Authenticate (Section 11.7.1 of [HTTP]), Proxy-Authentication-Info
            (Section 11.7.3 of [HTTP]), and Proxy-Authorization (Section 11.7.2 of [HTTP]).

    somehow need to update stored headers
        how - should read below
    generate Age header that equals to current_age of response, if the response has not been revalidated - done
    can only take response from cache if the request method is GET or HEAD (what about TRACE and OPTIONS?) - done
    unsafe requests must invalidate responses for those resources - done

    VARY:
        if a header is absent in a request - it can only match another (cached) request where the header is absent also - done, but needs to be tested
        split header into individual values - then compare (maintaining order!) - done, but needs to be tested (if a request comes with a single multivalue header - will it be split, or remain one?)
        make sure that when VARY specified a header that is not present in the request - the header is still a part of the cache key - done, but needs to be tested

    if the request contains "IF-Match" or "If-Unmodified-Since" - just pass the request to the origin - done
    (this requirement is not in the standard, and according to https://httpwg.org/specs/rfc9110.html#precedence, it looks like
    caches should disregard these headers at all!)

    if-none-match, if-modified-since - if a request is received with one of these - implement logic to validate locally, - done
        if no stored responses match - pass through, and handle the response - done

        freshening stored responses after validation (validation that was initiated downstream)

    e-tag, lastmodified - compare against existing values and invalidate if they do not match

    inalidate stored response if an unsafe request is made (or it has an unknown verb)

    process max-age header in the response

    authorization header hashing/salting

    for the time being, if any of cache-significant headers are in trailers - do not cache

    check precedence or preconditions:
        https://httpwg.org/specs/rfc9110.html#precedence
Later:
    double-keying

    validation of stale responses
    refresh expiration when 304 not modified is received?
    if no-cache - then maybe keep, but consider stale and impl revalidation (but be careful - it can have arguments that prevents storage all caches)
    if private - it might allow to store something?
    how to sync cache state (invalidation, staleness) across nodes?
    how to sync cache state (invalidation, staleness) for the responses that are still being loaded?
    partial content
    cache extensions
    heuristic staleness?
    responses that are allowed to be served stale?
    VARY header fields normalization:
        https://httpwg.org/specs/rfc9111.html#caching.negotiated.responses: normalizing both header field values in a way that is known to have identical semantics, according to the header field's specification (e.g., reordering field values when order is not significant; case-normalization, where values are defined to be case-insensitive)

    if multiple cached responses match the request - the most recent one (according to Date field)  must be used
    max-age and min-fresh request directives sent by client (they are recommendation-level)

    invalidate stored response if an unsafe request is made and the response contains Location and Content-location headers (but in the same origin only!)

    process max-stale, min-fresh, only-if-cached in request

    no-transform - ???

    proctess must-revalidate, must-understand + no-store, no-cache, no-transform (mandatory revalidation), proxy-revalidate in response

    figure out what to do when last-modified is considered "weak": https://httpwg.org/specs/rfc9110.html#lastmod.comparison
 */

// TODO: check that headers are handled as case-insensitive as expected

/// TODO
#[derive(Debug)]
pub struct CachingService<InnerService, Storage>
where
    Storage: CacheStorage + Send + Sync + 'static,
{
    inner: InnerService,
    storage: Storage,
}

impl<InnerService, Storage> CachingService<InnerService, Storage>
where
    Storage: CacheStorage + Send + Sync + 'static,
{
    /// Creates a new caching service
    pub fn new(inner: InnerService, storage: Storage) -> CachingService<InnerService, Storage>
    where
        Storage: CacheStorage + Send + Sync + 'static,
    {
        return CachingService { inner, storage };
    }

    async fn find_in_cache<ResponseBody, RequestBody, S>(
        &self,
        head: &http::request::Parts,
        req_cache_control: Option<&CacheControl>,
    ) -> Option<Response<UnionBody<<Storage as CacheStorage>::CachedResponseBody, ByteBody>>>
    where
        InnerService: Service<S, Request<RequestBody>, Response = Response<ResponseBody>>,
        RequestBody: Body + Send + 'static,
        ResponseBody:
            Body<Data: Buf + Clone + Send + Sync + 'static, Error: Into<BoxError>> + Send + 'static,
        S: Send + Sync + 'static,
    {
        if !Self::can_read_from_cache(head, req_cache_control) {
            return None;
        }

        // TODO: log failure, log not found result
        let Ok(Some(response)) = self.storage.get_response(head).await else {
            return None;
        };

        let (mut res_head, res_body) = response.0.into_parts();

        // TODO: check staleness before checking preconditions

        match handle_preconditions(head, &res_head, &response.1) {
            PreconditionHandleResult::Passed
            | PreconditionHandleResult::Invalid
            | PreconditionHandleResult::NotPresent => {}
            PreconditionHandleResult::NotPassed(res) => {
                let (head, body) = res.into_parts();
                return Some(Response::from_parts(head, UnionBody::second(body)));
            }
            PreconditionHandleResult::InternalError => {
                return None;
            }
        };

        let new_age_header = get_current_response_age(&response.1, &res_head)?;
        res_head.headers.remove("Age");
        res_head.headers.append("Age", new_age_header);

        return Some(Response::from_parts(res_head, UnionBody::first(res_body)));
    }

    async fn store_in_cache<ResponseBody, RequestBody, S>(
        &self,
        req_head: http::request::Parts,
        res_head: http::response::Parts,
        res_body: ResponseBody,
        req_time: &SystemTime,
        res_time: &SystemTime,
        req_cache_control: Option<&CacheControl>,
        res_cache_control: Option<&CacheControl>,
    ) -> Result<
        Response<<Storage as CacheStorage>::InterceptedResponseBody<ResponseBody>>,
        (http::request::Parts, http::response::Parts, ResponseBody),
    >
    where
        InnerService: Service<S, Request<RequestBody>, Response = Response<ResponseBody>>,
        RequestBody: Body + Send + 'static,
        ResponseBody:
            Body<Data: Buf + Clone + Send + Sync + 'static, Error: Into<BoxError>> + Send + 'static,
        S: Send + Sync + 'static,
    {
        if !Self::should_store(&req_head, &res_head, req_cache_control, res_cache_control) {
            return Err((req_head, res_head, res_body));
        }

        let Some(cache_key) = CacheKey::try_from_req_res(&req_head, &res_head) else {
            return Err((req_head, res_head, res_body));
        };

        let Some(expiration) = get_expiration_time(req_time, res_time, &res_head) else {
            return Err((req_head, res_head, res_body));
        };

        if expiration < SystemTime::now() {
            return Err((req_head, res_head, res_body));
        }

        let mut res_headers_to_store = res_head.clone();
        strip_non_storable_headers(&mut res_headers_to_store.headers);

        let metadata = &[
            (
                RESPONSE_DATE_KEY,
                &Into::<DateTime<Utc>>::into(res_time.clone()).to_rfc3339()[..],
            ),
            (
                REQUEST_DATE_KEY,
                &Into::<DateTime<Utc>>::into(req_time.clone()).to_rfc3339()[..],
            ),
        ];

        let cache_store_result = self
            .storage
            .set_response(
                Cow::Owned(cache_key),
                expiration,
                metadata,
                Response::from_parts(res_headers_to_store, res_body),
            )
            .await;

        // TODO: log unsuccessful save to cache
        match cache_store_result {
            Ok(intercepted_response) => {
                let (_, intercepted_body) = intercepted_response.into_parts();

                return Ok(Response::from_parts(res_head, intercepted_body));
            }
            Err((original_res, _error)) => {
                let (original_res_head, original_res_body) = original_res.into_parts();
                return Err((req_head, original_res_head, original_res_body));
            }
        };
    }

    fn should_purge(request_head: &request::Parts) -> bool {
        return !request_head.method.is_safe();
    }

    fn can_read_from_cache(
        request_head: &request::Parts,
        req_cache_control: Option<&CacheControl>,
    ) -> bool {
        if !Self::can_serve_method_from_cache(&request_head.method)
            || Self::is_range_request(request_head)
            || request_head.headers.contains_key("If-Match")
            || request_head.headers.contains_key("If-Unmodified-Since")
            // range requests not supported for the time being
            || request_head.headers.contains_key("Range")
            || request_head.headers.contains_key("If-Range")
        {
            return false;
        }

        let Some(req_cache_control) = req_cache_control else {
            return true;
        };

        if req_cache_control.no_cache() {
            return false;
        }

        return true;
    }

    fn can_serve_method_from_cache(method: &Method) -> bool {
        // TODO: figure out how to deal with HEAD requests
        return method == Method::GET;
    }

    fn is_range_request(request_head: &request::Parts) -> bool {
        // TODO: range requests not supported for the time being
        return request_head.headers.contains_key("Range");
    }

    fn should_store(
        request_head: &request::Parts,
        response_head: &response::Parts,
        req_cache_control: Option<&CacheControl>,
        res_cache_control: Option<&CacheControl>,
    ) -> bool {
        if request_head.method != Method::GET || response_head.status != StatusCode::OK {
            return false;
        }

        if let Some(req_cache_control) = req_cache_control {
            if req_cache_control.no_cache() || req_cache_control.no_store() {
                return false;
            }
        }

        if res_cache_control
            .map(|res_cc| res_cc.no_cache() || res_cc.no_store() || res_cc.private())
            .unwrap_or(false)
        {
            return false;
        }

        if request_head.headers.contains_key("Authorization")
            && !can_cache_with_auth(res_cache_control)
        {
            return false;
        }

        return true;

        fn can_cache_with_auth(res_cache_control: Option<&CacheControl>) -> bool {
            return res_cache_control
                // TODO: headers is missing must_revalidate - will need to add
                .map(
                    |res_cc| res_cc.public() || res_cc.s_max_age().is_some() /*|| res_cc.must_revalidate()*/
                )
                .unwrap_or(false);
        }
    }
}

impl<InnerService, Storage, S, RequestBody, ResponseBody> Service<S, Request<RequestBody>>
    for CachingService<InnerService, Storage>
where
    InnerService: Service<S, Request<RequestBody>, Response = Response<ResponseBody>>,
    RequestBody: Body + Send + 'static,
    ResponseBody:
        Body<Data: Buf + Clone + Send + Sync + 'static, Error: Into<BoxError>> + Send + 'static,
    Storage: CacheStorage + Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    type Response = Response<
        CacheResponseBodyVariant<
            ResponseBody,
            Storage::CachedResponseBody,
            Storage::InterceptedResponseBody<ResponseBody>,
        >,
    >;

    type Error = InnerService::Error;

    fn serve(
        &self,
        ctx: Context<S>,
        req: Request<RequestBody>,
    ) -> impl std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + '_ {
        return async {
            let (head, body) = req.into_parts();
            let req_cache_control =
                CacheControl::decode(&mut head.headers.get_all("Cache-Control").iter()).ok();

            if Self::should_purge(&head) {
                // TODO: log invalidation failure
                self.storage.invalidate_url(&head.uri).await.unwrap_or(());
            } else if let Some(response_from_cache) =
                self.find_in_cache(&head, req_cache_control.as_ref()).await
            {
                let (cached_head, cached_body) = response_from_cache.into_parts();

                return Ok(Response::from_parts(
                    cached_head,
                    match cached_body.into_variant() {
                        UnionBodyVariant::First(cached_response_body) => {
                            CacheResponseBodyVariant::new_cached_response(cached_response_body)
                        }
                        UnionBodyVariant::Second(byte_body) => {
                            CacheResponseBodyVariant::new_byte_response(byte_body)
                        }
                    },
                ));
            }

            let req_head = head.clone();
            let request_time = SystemTime::now();
            let response = self
                .inner
                .serve(ctx, Request::from_parts(head, body))
                .await?;
            let response_time = SystemTime::now();

            let (res_head, res_body) = response.into_parts();
            let res_cache_control =
                CacheControl::decode(&mut res_head.headers.get_all("Cache-Control").iter()).ok();

            match self
                .store_in_cache(
                    req_head,
                    res_head,
                    res_body,
                    &request_time,
                    &response_time,
                    req_cache_control.as_ref(),
                    res_cache_control.as_ref(),
                )
                .await
            {
                Ok(intercepted_response) => {
                    let (intercpeted_head, intercepted_body) = intercepted_response.into_parts();

                    return Ok(Response::from_parts(
                        intercpeted_head,
                        CacheResponseBodyVariant::new_intercepted_response(intercepted_body),
                    ));
                }
                Err((_, res_head, res_body)) => {
                    return Ok(Response::from_parts(
                        res_head,
                        CacheResponseBodyVariant::new_original_response(res_body),
                    ));
                }
            }
        };
    }
}

// TODO: implement this for trailers too (but doublecheck if this is really necessary)
fn strip_non_storable_headers(headers: &mut HeaderMap<HeaderValue>) {
    strip_connection_header(headers);

    for non_stored_header in NON_STORED_HEADERS {
        let Entry::Occupied(entry) = headers.entry(*non_stored_header) else {
            continue;
        };
        entry.remove_entry();
    }
}

fn strip_connection_header(headers: &mut HeaderMap<HeaderValue>) {
    let connection_header_names: Vec<HeaderName> = headers
        .get_all("Connection")
        .iter()
        .filter_map(|header_value| HeaderName::from_bytes(header_value.as_bytes()).ok())
        .collect();

    for connection_header_name in connection_header_names {
        let Entry::Occupied(entry) = headers.entry(connection_header_name) else {
            continue;
        };
        entry.remove_entry();
    }

    let Entry::Occupied(entry) = headers.entry("Connection") else {
        return;
    };
    entry.remove_entry();
}

// TODO: proper errors
fn get_current_response_age(
    metadata: &Vec<(String, String)>,
    res_head: &response::Parts,
) -> Option<HeaderValue> {
    let Some(Ok(request_date)) = metadata
        .iter()
        .find(|t| t.1 == REQUEST_DATE_KEY)
        .map(|t| DateTime::<Utc>::from_str(&t.1))
    else {
        return None;
    };
    let Some(Ok(response_date)) = metadata
        .iter()
        .find(|t| t.0 == RESPONSE_DATE_KEY)
        .map(|t| DateTime::<Utc>::from_str(&t.1))
    else {
        return None;
    };
    let current_age = get_current_age(&GetAgeParams::new(
        request_date.into(),
        response_date.into(),
        res_head,
    ))?;
    let Ok(new_age_header) = HeaderValue::from_str(&current_age.as_secs().to_string()) else {
        return None;
    };
    Some(new_age_header)
}

fn handle_preconditions(
    req_head: &request::Parts,
    cached_res_head: &response::Parts,
    cache_res_metadata: &Vec<(String, String)>,
) -> PreconditionHandleResult {
    if req_head.headers.contains_key("If-None-Match") {
        return handle_if_none_match_req(req_head, cached_res_head);
    } else if (req_head.method == Method::GET || req_head.method == Method::HEAD)
        && req_head.headers.get_all("If-Modified-Since").iter().count() == 1
    {
        return handle_if_modified_since_req(req_head, cached_res_head, cache_res_metadata);
    }

    return PreconditionHandleResult::NotPresent;
}

fn handle_if_none_match_req(
    req_head: &request::Parts,
    cached_res_head: &response::Parts,
) -> PreconditionHandleResult {
    let Ok(stored_etag) = ETag::decode(&mut cached_res_head.headers.get_all("ETag").iter()) else {
        return PreconditionHandleResult::Invalid;
    };

    let Ok(req_if_none_match) =
        IfNoneMatch::decode(&mut req_head.headers.get_all("If-None-Match").iter())
    else {
        return PreconditionHandleResult::Invalid;
    };

    if req_if_none_match.precondition_passes(&stored_etag) {
        return PreconditionHandleResult::Passed;
    }

    let Some(generated_304_response) = generate_304_response(cached_res_head) else {
        return PreconditionHandleResult::InternalError;
    };

    return PreconditionHandleResult::NotPassed(generated_304_response);
}

fn handle_if_modified_since_req(
    req_head: &request::Parts,
    cached_res_head: &response::Parts,
    cache_res_metadata: &Vec<(String, String)>,
) -> PreconditionHandleResult {
    let Some(cache_res_last_modified) =
        get_cache_res_last_modified(cached_res_head, cache_res_metadata)
    else {
        return PreconditionHandleResult::InternalError;
    };

    let Ok(req_if_modified_since) =
        IfModifiedSince::decode(&mut req_head.headers.get_all("If-Modified-Since").iter())
    else {
        return PreconditionHandleResult::Invalid;
    };

    if SystemTime::from(req_if_modified_since) < cache_res_last_modified {
        return PreconditionHandleResult::Passed;
    }

    let Some(generated_304_response) = generate_304_response(cached_res_head) else {
        return PreconditionHandleResult::InternalError;
    };

    return PreconditionHandleResult::NotPassed(generated_304_response);

    fn get_cache_res_last_modified(
        cached_res_head: &response::Parts,
        cache_res_metadata: &Vec<(String, String)>,
    ) -> Option<SystemTime> {
        if let Ok(last_modified) =
            LastModified::decode(&mut cached_res_head.headers.get_all(LastModified::name()).iter())
        {
            return Some(last_modified.into());
        }

        if let Ok(date) = Date::decode(&mut cached_res_head.headers.get_all(Date::name()).iter()) {
            return Some(date.into());
        }

        if let Some(response_date) = cache_res_metadata
            .iter()
            .find(|t| t.0 == RESPONSE_DATE_KEY)
            .map(|res_date_str| DateTime::<FixedOffset>::parse_from_rfc3339(&res_date_str.1).ok())
            .flatten()
            .map(|res_date_time| SystemTime::from(res_date_time))
        {
            return Some(response_date);
        }

        return None;
    }
}

fn generate_304_response(parts: &response::Parts) -> Option<Response<ByteBody>> {
    let mut response_builder = response::Builder::new().status(304).version(parts.version);

    for &header_name in NOT_MODIFIED_RESPONSE_HEADERS {
        for header_value in parts.headers.get_all(header_name) {
            response_builder = response_builder.header(header_name, header_value);
        }
    }

    return response_builder
        .body(ByteBody::from_byte_vec(&Vec::new())?)
        .ok();
}

enum PreconditionHandleResult {
    NotPresent,
    Passed,
    NotPassed(Response<ByteBody>),
    Invalid,
    InternalError,
}

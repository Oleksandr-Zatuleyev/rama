use std::{borrow::Cow, fmt::Debug};

use bytes::Buf;
use http::{Request, Response};
use http_body::Body;
use rama_core::{error::BoxError, Context, Service};

use crate::layer::util::union_body::{UnionBody, UnionBody3};

use super::{
    cache_key::CacheKey, cache_purge_predicate::CachePurgePredicate,
    cache_read_predicate::CacheReadPredicate, cache_store_predicate::CacheStorePredicate,
    CacheStorage,
};

/// TODO
#[derive(Debug)]
pub struct CachingService<InnerService, ReadPredicate, StorePredicate, PurgePredicate, Storage>
where
    ReadPredicate: CacheReadPredicate + Send + Sync + 'static,
    StorePredicate: CacheStorePredicate + Send + Sync + 'static,
    PurgePredicate: CachePurgePredicate + Send + Sync + 'static,
    Storage: CacheStorage + Send + Sync + 'static,
{
    inner: InnerService,
    read_predicate: ReadPredicate,
    store_predicate: StorePredicate,
    purge_predicate: PurgePredicate,
    storage: Storage,
}

impl<InnerService, ReadPredicate, StorePredicate, PurgePredicate, Storage>
    CachingService<InnerService, ReadPredicate, StorePredicate, PurgePredicate, Storage>
where
    ReadPredicate: CacheReadPredicate + Send + Sync + 'static,
    StorePredicate: CacheStorePredicate + Send + Sync + 'static,
    PurgePredicate: CachePurgePredicate + Send + Sync + 'static,
    Storage: CacheStorage + Send + Sync + 'static,
{
    /// Creates a new caching service
    pub fn new(
        inner: InnerService,
        read_predicate: ReadPredicate,
        store_predicate: StorePredicate,
        purge_predicate: PurgePredicate,
        storage: Storage,
    ) -> CachingService<InnerService, ReadPredicate, StorePredicate, PurgePredicate, Storage>
    where
        ReadPredicate: CacheReadPredicate + Send + Sync + 'static,
        StorePredicate: CacheStorePredicate + Send + Sync + 'static,
        PurgePredicate: CachePurgePredicate + Send + Sync + 'static,
        Storage: CacheStorage + Send + Sync + 'static,
    {
        return CachingService {
            inner,
            read_predicate,
            store_predicate,
            purge_predicate,
            storage,
        };
    }
}

// TODO: cache control, revalidation etc
impl<
        InnerService,
        ReadPredicate,
        StorePredicate,
        PurgePredicate,
        Storage,
        S,
        RequestBody,
        ResponseBody,
    > Service<S, Request<RequestBody>>
    for CachingService<InnerService, ReadPredicate, StorePredicate, PurgePredicate, Storage>
where
    InnerService: Service<S, Request<RequestBody>, Response = Response<ResponseBody>>,
    RequestBody: Body + Send + 'static,
    ResponseBody:
        Body<Data: Buf + Clone + Send + Sync + 'static, Error: Into<BoxError>> + Send + 'static,
    ReadPredicate: CacheReadPredicate + Send + Sync + 'static,
    StorePredicate: CacheStorePredicate + Send + Sync + 'static,
    PurgePredicate: CachePurgePredicate + Send + Sync + 'static,
    Storage: CacheStorage + Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    type Response = Response<UnionBody3<ResponseBody, Storage::CachedResponseBody, Storage::InterceptedResponseBody<ResponseBody>>>;

    type Error = InnerService::Error;

    fn serve(
        &self,
        ctx: Context<S>,
        req: Request<RequestBody>,
    ) -> impl std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + '_ {
        return async {
            let (head, body) = req.into_parts();

            if self.purge_predicate.should_purge(&head) {
                // TODO: log invalidation failure
                self.storage
                    .invalidate_response(&head.uri)
                    .await
                    .unwrap_or(());
            } else if self.read_predicate.can_read_from_cache(&head) {
                // TODO: log failure, log not found result
                if let Ok(Some(response)) = self.storage.get_response(&head).await {
                    let (res_head, res_body) = response.into_parts();
                    return Ok(Response::from_parts(res_head, UnionBody::second_of_3(res_body)));
                }
            }

            let req_head = head.clone();
            let response = self
                .inner
                .serve(ctx, Request::from_parts(head, body))
                .await?;

            let (res_head, res_body) = response.into_parts();

            if self.store_predicate.should_store(&req_head, &res_head) {
                // TODO: is there an elegant way to avoid these nested ifs?
                if let Some(cache_key) = CacheKey::try_from_req_res(&req_head, &res_head) {
                    let cache_store_result = self
                        .storage
                        .set_response(
                            Cow::Owned(cache_key),
                            Response::from_parts(res_head, res_body),
                        )
                        .await;

                    // TODO: log unsuccessful save to cache
                    let result_body = match cache_store_result {
                        Ok(intercepted_response) => {
                            let (intercpeted_head, intercepted_body) = intercepted_response.into_parts();

                            return Ok(Response::from_parts(
                                intercpeted_head,
                                UnionBody::third_of_3(intercepted_body),
                            ));
                        }
                        Err((original_res, _error)) => {
                            let (original_res_head, original_res_body) = original_res.into_parts();
                            return Ok(Response::from_parts(
                                original_res_head,
                                UnionBody::first_of_3(original_res_body),
                            ));
                        }
                    };
                }
            }

            return Ok(Response::from_parts(
                res_head,
                UnionBody::first_of_3(res_body)
            ));
        };
    }
}

// // root node:
// // 1) url - key
// // 2) list of variance headers. N.B: how to deal with auth headers?
// // 3) number of data nodes (to remove the root node once no data nodes are present)
// //
// // data node:
// // 1) list of variance header names
// // 2) expiration info
// // 3) guid to lru cache

// use std::{
//     collections::HashMap,
//     sync::{Arc, Mutex, Weak},
// };

// use bytes::Bytes;
// use http::Uri;
// use http_body::Body;
// use lru::LruCache;

// use super::{CacheKey, CacheStorage};

// /// In Memory storage for cached http response that uses LRU eviction strategy
// #[derive(Debug)]
// pub struct MemoryCacheStorage {
//     state: Arc<Mutex<MemoryCacheStorageState>>,
// }

// #[derive(Debug)]
// struct MemoryCacheStorageState {
//     resources: HashMap<Uri, CacheRootNode>,
//     lru: LruCache<CacheKey, CacheDataNode>,
//     max_size: usize,
//     current_size: usize,
// }

// #[derive(Debug)]
// struct CacheRootNode {
//     uri: Uri,
//     data_nodes: usize,
// }

// #[derive(Debug)]
// struct CacheDataNode {
//     response_head: http::response::Parts,
//     body_data: CachedBodyData, // TODO: expiration
// }

// impl CacheStorage for MemoryCacheStorage {
//     type CachedResponseBody = CachedBody;

//     fn get_response(
//         &self,
//         request_head: &http::request::Parts,
//     ) -> impl std::future::Future<
//         Output = Result<
//             Option<rama_http_types::Response<Self::CachedResponseBody>>,
//             rama_core::error::BoxError,
//         >,
//     > + Send
//            + 'static {
//         // let state =

//         async {
//             todo!();
//         }
//     }

//     fn set_response(
//         &self,
//         key: std::borrow::Cow<'_, super::CacheKey>,
//         response: rama_http_types::Response<impl http_body::Body>,
//     ) -> impl std::future::Future<Output = Result<(), rama_core::error::BoxError>> + Send + 'static
//     {
//         async {
//             todo!();
//         }
//     }

//     fn invalidate_response(
//         &self,
//         uri: &http::Uri,
//     ) -> impl std::future::Future<Output = Result<(), rama_core::error::BoxError>> + Send + 'static
//     {
//         async {
//             todo!();
//         }
//     }
// }

// fn get_estimated_size(
//     key: std::borrow::Cow<'_, super::CacheKey>,
//     response_head: &http::response::Parts,
//     body: Bytes,
// ) -> usize {
//     todo!();
// }

// #[derive(Debug)]
// enum CachedBodyData {
//     Incomplete(Mutex<IncompleteData>),
//     Complete(Bytes),
//     Failed,
// }

// #[derive(Debug)]
// struct IncompleteData {
//     listeners: Option<Vec<Weak<CachedBody>>>,
//     bytes: Vec<Bytes>,
// }

// pub struct CachedBody {}

// impl Body for CachedBody {
//     type Data = Bytes;

//     type Error = rama_core::error::BoxError;

//     fn poll_frame(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
//         todo!()
//     }
// }

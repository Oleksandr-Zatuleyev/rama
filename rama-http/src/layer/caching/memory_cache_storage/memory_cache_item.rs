use std::{collections::HashMap, sync::RwLock, time::SystemTime};

use bytes::Bytes;
use rama_core::context::Extensions;
use rama_http_types::{HeaderMap, StatusCode};
use uuid::Uuid;

use crate::layer::caching::CacheKey;

pub(super) struct MemoryCacheItem {
    // cache_key is needed in value to doublecheck key match, because tinyufo compares by u64 hash, not by the actual value
    pub(super) cache_key: CacheKey,
    pub(super) path_version: Uuid,
    pub(super) body_id: Uuid,
    pub(super) body: Vec<Bytes>,
    pub(super) status: StatusCode,

    pub(super) mutable_state: RwLock<MemoryCacheItemMutableState>
}

pub(super) struct MemoryCacheItemMutableState {
    pub(super) headers: HeaderMap,
    pub(super) extensions: Extensions,
    pub(super) trailers: HeaderMap,
    pub(super) metadata: HashMap<String, String>,
    pub(super) expiration: SystemTime
}

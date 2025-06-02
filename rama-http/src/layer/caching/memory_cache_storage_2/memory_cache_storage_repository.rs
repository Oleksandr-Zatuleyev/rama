use core::sync;
use std::{
    cmp::Reverse,
    collections::HashMap,
    future,
    ops::Deref,
    pin::{Pin, pin},
    sync::{Arc, RwLock, Weak, atomic::AtomicU32},
    time::{Duration, SystemTime},
};

use bytes::Bytes;
use dashmap::DashMap;
use keyed_priority_queue::KeyedPriorityQueue;
use rama_core::context::Extensions;
use rama_http_types::{HeaderMap, StatusCode, Uri};
use tinyufo::TinyUfo;
use tokio::{
    select,
    sync::{
        mpsc::{self},
        oneshot,
    },
    time::sleep,
};
use uuid::Uuid;

use crate::layer::caching::{CacheKey, cache_key::HeaderKey};

use super::{MemoryCacheItem, memory_cache_item::MemoryCacheItemMutableState};

pub(super) struct MemoryCacheStorageRepository {
    cache: TinyUfo<CacheKey, Arc<MemoryCacheItem>>,
    path_map: DashMap<Uri, PathMapItem>,
    expiration_management: RwLock<Option<ExpirationManagement>>,
    self_ref: Weak<MemoryCacheStorageRepository>,
}

struct ExpirationManagement {
    message_sender: mpsc::Sender<ExpirationManagementMessage>,
    on_destroy: Option<oneshot::Sender<()>>,
}

impl MemoryCacheStorageRepository {
    pub(super) fn new(total_weight_limit: usize) -> Arc<Self> {
        return Arc::new_cyclic(|self_ref| {
            return MemoryCacheStorageRepository {
                // TODO: figure out if it is acceptable to use total_weight_limit in both positions
                cache: TinyUfo::new(total_weight_limit, total_weight_limit),
                path_map: DashMap::new(),
                expiration_management: RwLock::new(None),
                self_ref: self_ref.clone(),
            };
        });
    }

    pub(super) fn get_item(
        &self,
        path: &Uri,
        req_headers: &HeaderMap,
    ) -> Option<Arc<MemoryCacheItem>> {
        let path_version;
        let cache_key;

        {
            let path_map_item = self.path_map.get(path)?;

            cache_key = CacheKey::from_req_uri_headers_variance(
                path,
                req_headers,
                Some(
                    path_map_item
                        .vary_headers
                        .iter()
                        .map(|vary_header| vary_header.clone()),
                ),
            );

            path_version = path_map_item.deref().version;
        };

        let cache_item = self.cache.get(&cache_key)?;

        if cache_item.path_version != path_version {
            // TODO: log path version mismatch
            return None;
        }

        if cache_item.cache_key != cache_key {
            // TODO: log hash collision
            return None;
        }

        return Some(cache_item);
    }

    pub(super) fn add_item(&self, params: AddItemParams) -> impl Future<Output = bool> + Send + '_ {
        async move {
            // TODO: this limitation might be a bit too harsh, becaue it will not allow us to cache items larger than 64KiB
            let Ok(weight) = params.content_length.try_into() else {
                return false;
            };

            // TODO: too many resources are being spent on something that may be immediately dropped
            let path_version = self.register_path_use(&params.cache_key);
            let mut item_accepted = true;
            let body_id = Uuid::new_v4();

            let new_item = Arc::new(MemoryCacheItem {
                cache_key: params.cache_key,
                path_version: path_version,
                body_id: body_id,
                body: params.body,
                status: params.status,
                mutable_state: RwLock::new(MemoryCacheItemMutableState {
                    headers: params.headers,
                    extensions: params.extensions,
                    trailers: params.trailers,
                    metadata: params.metadata,
                    expiration: params.expiration,
                }),
            });

            let dropped_items =
                self.cache
                    .put(new_item.cache_key.clone(), Arc::clone(&new_item), weight);

            let expiration_management_sender = self.ensure_expiration_queue();

            for dropped_item in dropped_items {
                let dropped_item = dropped_item.data.deref();

                if body_id == dropped_item.body_id {
                    item_accepted = false;
                }

                self.unregister_dropped_item(dropped_item);

                if body_id != dropped_item.body_id {
                    expiration_management_sender
                        .send(ExpirationManagementMessage::Removed {
                            cache_key: dropped_item.cache_key.clone(),
                            body_version: dropped_item.body_id,
                        })
                        .await
                        .ok();
                }
            }

            if item_accepted {
                expiration_management_sender
                    .send(ExpirationManagementMessage::Upserted {
                        cache_key: new_item.cache_key.clone(),
                        body_version: new_item.body_id,
                        expiration: params.expiration,
                    })
                    .await
                    .ok();
            }

            return item_accepted;
        }
    }

    pub(super) fn invalidate_url(&self, uri: &Uri) {
        self.path_map.remove(uri);
    }

    pub(super) fn invalidate_item(&self, cache_key: &CacheKey, _body_version: Uuid) {
        // TODO: what if the key is already being occupied by another body with the same key?
        // we should check body_id, but it is not possible to do so without calling get on cache,
        // but it will add incorrect data to cache statistics, and TinyUfo does not support peek
        let Some(dropped_cache_item) = self.cache.remove(cache_key) else {
            return;
        };

        self.unregister_dropped_item(&dropped_cache_item);
    }

    // TODO: refactor
    pub(super) fn notify_cache_item_updated(
        &self,
        cache_item: &MemoryCacheItem,
    ) -> impl Future<Output = ()> + Send + '_ {
        let expiration = { cache_item.mutable_state.read().unwrap().deref().expiration };
        let message = ExpirationManagementMessage::Upserted {
            cache_key: cache_item.cache_key.clone(),
            body_version: cache_item.body_id,
            expiration,
        };

        async move {
            let expiration_management = self.ensure_expiration_queue();

            expiration_management
                .send(message)
                .await;
        }
    }

    fn register_path_use(&self, cache_key: &CacheKey) -> Uuid {
        return self
            .path_map
            .entry(cache_key.get_uri().clone())
            .and_modify(|existing| {
                let new_vary_headers = cache_key.get_variance_headers();

                if !existing.vary_headers.iter().eq(new_vary_headers) {
                    // TODO: log variance change
                    *existing = Self::get_new_path_map_item(&cache_key);
                }

                existing
                    .ref_counter
                    .fetch_add(1, sync::atomic::Ordering::Relaxed);
            })
            .or_insert_with(|| {
                let new_item = Self::get_new_path_map_item(&cache_key);
                new_item
                    .ref_counter
                    .fetch_add(1, sync::atomic::Ordering::Relaxed);
                return new_item;
            })
            .version;
    }

    fn get_new_path_map_item(cache_key: &CacheKey) -> PathMapItem {
        return PathMapItem {
            vary_headers: cache_key
                .get_variance_headers()
                .map(|variance_header| variance_header.clone())
                .collect(),
            version: Uuid::new_v4(),
            ref_counter: AtomicU32::new(0),
        };
    }

    fn unregister_dropped_item(&self, dropped_item: &MemoryCacheItem) {
        self.path_map
            .remove_if_mut(dropped_item.cache_key.get_uri(), |_, path_map_item| {
                if dropped_item.path_version != path_map_item.version {
                    return false;
                }

                let prev_ref_counter = path_map_item
                    .ref_counter
                    .fetch_sub(1, sync::atomic::Ordering::Relaxed);

                return prev_ref_counter == 1;
            });
    }

    // TODO: how to avoid sender cloning?
    fn ensure_expiration_queue(&self) -> mpsc::Sender<ExpirationManagementMessage> {
        {
            let read_g = self.expiration_management.read().unwrap();

            if let Some(expiration_management) = read_g.deref() {
                return expiration_management.message_sender.clone();
            }
        }

        let mut write_g = self.expiration_management.write().unwrap();

        if let Some(expiration_management) = write_g.deref() {
            return expiration_management.message_sender.clone();
        }

        let (drop_sender, drop_receiver) = oneshot::channel();

        let (sender, receiver) = mpsc::channel(10000);

        *write_g = Some(ExpirationManagement {
            message_sender: sender.clone(),
            on_destroy: Some(drop_sender),
        });

        start_expiration_queue(self.self_ref.clone(), receiver, drop_receiver);

        return sender;
    }
}

impl Drop for ExpirationManagement {
    fn drop(&mut self) {
        let Some(on_destroy) = self.on_destroy.take() else {
            return;
        };

        on_destroy.send(()).ok();
    }
}

pub(super) struct AddItemParams {
    pub cache_key: CacheKey,
    pub body: Vec<Bytes>,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub extensions: Extensions,
    pub trailers: HeaderMap,
    pub metadata: HashMap<String, String>,
    pub expiration: SystemTime,
    pub content_length: u64,
}

struct PathMapItem {
    vary_headers: Vec<HeaderKey>,
    // TODO: this it not version - this is id
    version: Uuid,
    ref_counter: AtomicU32,
}

fn start_expiration_queue(
    repo: Weak<MemoryCacheStorageRepository>,
    mut message_receiver: mpsc::Receiver<ExpirationManagementMessage>,
    on_drop: oneshot::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut on_drop_pin = pin!(on_drop);

        let mut queue: KeyedPriorityQueue<(CacheKey, Uuid), Reverse<SystemTime>> =
            KeyedPriorityQueue::new();

        loop {
            let next_expiration = queue.peek();

            let delay_fut: Pin<Box<dyn Future<Output = &(CacheKey, Uuid)> + Send>> =
                match next_expiration {
                    Some(expiration) => Box::pin(async move {
                        let now = SystemTime::now();

                        let delay = expiration
                            .1
                            .0
                            .duration_since(now)
                            .ok()
                            .unwrap_or(Duration::from_secs(0));

                        sleep(delay).await;
                        return expiration.0;
                    }),
                    None => Box::pin(future::pending()),
                };

            select! {
                expiration = delay_fut => {
                    let Some(repo) = repo.upgrade() else {
                        return;
                    };

                    let repo = repo.deref();

                    repo.invalidate_item(&expiration.0, expiration.1);
                }
                received = message_receiver.recv() => {
                    match received {
                        Some(ExpirationManagementMessage::Upserted {
                            cache_key,
                            body_version,
                            expiration,
                        }) => {
                            queue.push((cache_key, body_version), Reverse(expiration));
                        }
                        Some(ExpirationManagementMessage::Removed {
                            cache_key,
                            body_version,
                        }) => {
                            queue.remove(&(cache_key, body_version));
                        }
                        None => return,
                    };
                }
                _ = on_drop_pin.as_mut() => {
                    return;
                }
            };
        }
    });
}

pub(super) enum ExpirationManagementMessage {
    Upserted {
        cache_key: CacheKey,
        body_version: Uuid,
        expiration: SystemTime,
    },

    Removed {
        cache_key: CacheKey,
        body_version: Uuid,
    },
}

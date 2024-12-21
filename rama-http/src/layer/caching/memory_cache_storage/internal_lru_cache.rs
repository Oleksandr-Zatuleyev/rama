use std::num::NonZeroUsize;

use lru::LruCache;

use crate::layer::caching::CacheKey;

pub(super) struct LruStorage<Data> {
    max_size: usize,
    current_size: usize,
    lru: LruCache<CacheKey, LruItem<Data>>,
}

impl<Data> LruStorage<Data> {
    pub(super) fn new(max_size: usize) -> LruStorage<Data> {
        return LruStorage {
            max_size,
            current_size: 0,
            lru: LruCache::new(NonZeroUsize::MAX),
        };
    }

    pub(super) fn get_item(&mut self, key: &CacheKey) -> Option<&mut Data> {
        return Some(&mut self.lru.get_mut(key)?.data);
    }

    pub(super) fn peek_item(&mut self, key: &CacheKey) -> Option<&mut Data> {
        return Some(&mut self.lru.peek_mut(key)?.data);
    }

    pub(super) fn promote_item(&mut self, key: &CacheKey) {
        self.lru.promote(key);
    }

    pub(super) fn resize_item(&mut self, key: &CacheKey, new_size: usize) {
        let item = self.lru.peek_mut(key);

        let Some(item) = item else {
            return;
        };

        self.current_size = self.current_size - item.size + new_size;
        item.size = new_size;
    }

    pub(super) fn remove_item(&mut self, key: &CacheKey) {
        let Some(item) = self.lru.pop(key) else {
            return;
        };

        self.current_size -= item.size;
    }

    pub(super) fn set_item(&mut self, key: CacheKey, data: Data, size: usize) {
        let item = LruItem { data, size };

        let Some(previous_item) = self.lru.put(key.clone(), item) else {
            self.current_size += size;
            return;
        };

        self.current_size = self.current_size - previous_item.size + size;
    }

    pub(super) fn evict_one(&mut self) -> Option<CacheKey> {
        let evicted_item = self.lru.pop_lru()?;
        self.current_size -= evicted_item.1.size;
        return Some(evicted_item.0);
    }

    pub(super) fn is_size_exceeded(&self) -> bool {
        return self.current_size > self.max_size;
    }
}

struct LruItem<Data> {
    data: Data,
    size: usize,
}

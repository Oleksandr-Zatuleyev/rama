use std::time::Instant;

use keyed_priority_queue::KeyedPriorityQueue;

use crate::layer::caching::CacheKey;

pub(super) struct ExpirationQueue {
    queue: KeyedPriorityQueue<CacheKey, Instant>,
}

impl ExpirationQueue {
    pub(super) fn new() -> ExpirationQueue {
        return ExpirationQueue {
            queue: KeyedPriorityQueue::new(),
        };
    }

    pub(super) fn set_expiration(&mut self, key: CacheKey, expiration: Instant) {
        self.queue.push(key, expiration);
    }

    pub(super) fn get_expiration(&self, key: &CacheKey) -> Option<&Instant> {
        return self.queue.get_priority(key);
    }

    pub(super) fn remove_item(&mut self, key: &CacheKey) {
        self.queue.remove(key);
    }

    pub(super) fn peek_first_to_expire(&self) -> Option<(&CacheKey, &Instant)> {
        return self.queue.peek();
    }
}

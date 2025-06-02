use std::time::SystemTime;

use keyed_priority_queue::KeyedPriorityQueue;

use crate::layer::caching::CacheKey;

pub(crate) struct ExpirationQueue {
    queue: KeyedPriorityQueue<CacheKey, SystemTime>,
}

impl ExpirationQueue {
    pub(crate) fn new() -> ExpirationQueue {
        return ExpirationQueue {
            queue: KeyedPriorityQueue::new(),
        };
    }

    pub(crate) fn set_expiration(&mut self, key: CacheKey, expiration: SystemTime) {
        self.queue.push(key, expiration);
    }

    pub(crate) fn get_expiration(&self, key: &CacheKey) -> Option<&SystemTime> {
        return self.queue.get_priority(key);
    }

    pub(crate) fn remove_item(&mut self, key: &CacheKey) {
        self.queue.remove(key);
    }

    pub(crate) fn peek_first_to_expire(&self) -> Option<(&CacheKey, &SystemTime)> {
        return self.queue.peek();
    }
}

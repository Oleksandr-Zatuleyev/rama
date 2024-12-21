use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Deref,
};

use http::Uri;

use crate::layer::caching::{cache_key::HeaderKey, CacheKey};

pub(super) struct UriMap {
    items: HashMap<Uri, UriMapItem>,
}

impl UriMap {
    pub(super) fn new() -> UriMap {
        return UriMap {
            items: HashMap::new(),
        };
    }

    pub(super) fn add_key(
        &mut self,
        uri: Cow<'_, Uri>,
        key: CacheKey,
    ) -> Option<HashSet<CacheKey>> {
        let Some(mut existing) = self.items.get_mut(uri.deref()) else {
            let map_item = UriMapItem::from_cache_key(key);

            self.items.insert(uri.into_owned(), map_item);

            return None;
        };

        let has_variance_changed = match (&existing.variance, key.get_variance_headers()) {
            (None, None) => false,
            (Some(existing_variance), Some(new_variance_headers)) => {
                let mut new_header_count = 0;
                let mut all_new_headers_found = true;

                for new_header in new_variance_headers {
                    new_header_count += 1;
                    if !existing_variance.contains(new_header) {
                        all_new_headers_found = false;
                        break;
                    }
                }

                !all_new_headers_found || new_header_count != existing_variance.len()
            }
            _ => true,
        };

        if !has_variance_changed {
            existing.keys.insert(key);
            return None;
        }

        let mut swap_map_item = UriMapItem::from_cache_key(key);

        std::mem::swap(&mut swap_map_item, &mut existing);

        return Some(swap_map_item.keys);
    }

    pub(super) fn remove_key(&mut self, uri: &Uri, key: &CacheKey) {
        let Some(items) = self.items.get_mut(uri) else {
            return;
        };

        items.keys.remove(key);

        if items.keys.len() == 0 {
            self.remove(uri);
        }
    }

    pub(super) fn remove(&mut self, uri: &Uri) -> Option<HashSet<CacheKey>> {
        return self.items.remove(uri).map(|removed| removed.keys);
    }

    pub(super) fn get_keys(&self, uri: &Uri) -> Option<&HashSet<CacheKey>> {
        return self.items.get(uri).map(|item| &item.keys);
    }

    pub(super) fn get_variance(&self, uri: &Uri) -> Option<&HashSet<HeaderKey>> {
        return self.items.get(uri)?.variance.as_ref();
    }
}

struct UriMapItem {
    keys: HashSet<CacheKey>,
    variance: Option<HashSet<HeaderKey>>,
}

impl UriMapItem {
    fn from_cache_key(key: CacheKey) -> UriMapItem {
        let mut map_item = UriMapItem {
            variance: key
                .get_variance_headers()
                .map(|headers| headers.map(|header| header.clone()).collect()),
            keys: HashSet::new(),
        };
        map_item.keys.insert(key);
        return map_item;
    }
}

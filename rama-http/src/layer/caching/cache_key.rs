use std::{borrow::Borrow, collections::BTreeMap, hash::Hash};

use rama_http_types::{
    HeaderMap,
    dep::http::{Uri, request::Parts as ReqParts, response::Parts as ResParts},
};

/// TODO: docs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    uri: Uri,
    authorization: Option<Vec<u8>>,
    // BTreeMap is needed for stable hashcode
    // for comparison and hashcode purposes, BTreeMap cannot be empty - it must be None instead
    variance: Option<BTreeMap<HeaderKey, VarianceHeaderValue>>,
}

#[derive(Debug, Clone)]
pub enum VarianceHeaderValue {
    Single(Vec<u8>),
    Multiple(Vec<Vec<u8>>),
}

#[derive(Debug, Clone)]
pub enum HeaderKey {
    StaticStr(&'static str),
    String(String),
}

impl CacheKey {
    /// Creates a key for caching based on request and response, taking into account variance and Authorization
    pub fn try_from_req_res(req_head: &ReqParts, res_head: &ResParts) -> Option<CacheKey> {
        if !CacheKey::is_vary_processable(&res_head) {
            return None;
        }

        return Some(CacheKey {
            uri: req_head.uri.clone(),
            authorization: CacheKey::get_req_authorization(&req_head.headers),
            variance: CacheKey::get_req_res_variance(req_head, res_head),
        });
    }

    pub fn from_req_uri_headers_variance<Variance: Iterator<Item: Into<HeaderKey>>>(
        uri: &Uri,
        req_headers: &HeaderMap,
        variance: Option<Variance>,
    ) -> CacheKey {
        return CacheKey {
            uri: uri.clone(),
            authorization: Self::get_req_authorization(req_headers),
            variance: variance
                .map(|v| Self::get_req_variance_headers(&req_headers, v))
                .flatten(),
        };
    }

    pub fn from_req_variance<Variance>(req_head: &ReqParts, variance: Option<Variance>) -> CacheKey
    where
        Variance: Iterator<Item: Into<HeaderKey>>,
    {
        return Self::from_req_uri_headers_variance(&req_head.uri, &req_head.headers, variance);
    }

    // pub fn from_req(req_head: &ReqParts) -> CacheKey {
    //     return CacheKey {
    //         uri: req_head.uri.clone(),
    //         authorization: CacheKey::get_req_authorization(req_head),
    //         variance: None,
    //     };
    // }

    /// Gets the uri
    pub fn get_uri(&self) -> &Uri {
        return &self.uri;
    }

    /// Gets the sorted headers in the variance
    pub fn get_variance_headers(&self) -> impl Iterator<Item = &HeaderKey> {
        return self.variance.iter().flat_map(|variance_headers| variance_headers.keys());
    }

    /// Returns variance header value, if present
    /// Will return None otherwise
    pub fn get_variance_header_value(
        &self,
        header_key: &HeaderKey,
    ) -> Option<&VarianceHeaderValue> {
        return self.variance.as_ref()?.get(header_key);
    }

    /// Returns request authorization, if present
    pub fn get_authorization(&self) -> Option<&[u8]> {
        return self.authorization.as_deref();
    }

    fn get_req_authorization(req_headers: &HeaderMap) -> Option<Vec<u8>> {
        // TODO: store hash instead of the full Authorization value?
        return req_headers
            .get("Authorization")
            .map(|auth| auth.as_bytes().to_owned());
    }

    // fn get_req_res_variance(
    //     req_head: &ReqParts,
    //     res_head: &ResParts,
    // ) -> Option<BTreeMap<HeaderKey, VarianceHeaderValue>> {
    //     if !res_head.headers.contains_key("Vary") {
    //         return None;
    //     }

    //     let mut variance = BTreeMap::new();

    //     for vary_header_name in res_head
    //         .headers
    //         .get_all("Vary")
    //         .iter()
    //         .filter_map(|vary_header| vary_header.to_str().ok())
    //     {
    //         if variance.contains_key(vary_header_name) {
    //             continue;
    //         }

    //         let vary_values = req_head.headers.get_all(vary_header_name);

    //         let mut variance_value: Option<VarianceHeaderValue> = None;

    //         for vary_value in vary_values {
    //             let vary_value = vary_value.as_bytes().to_owned();

    //             variance_value = match variance_value {
    //                 None => Some(VarianceHeaderValue::Single(vary_value)),
    //                 Some(VarianceHeaderValue::Single(existing)) => {
    //                     Some(VarianceHeaderValue::Multiple(vec![existing, vary_value]))
    //                 }
    //                 Some(VarianceHeaderValue::Multiple(mut existing)) => {
    //                     existing.push(vary_value);
    //                     Some(VarianceHeaderValue::Multiple(existing))
    //                 }
    //             };
    //         }

    //         variance.insert(
    //             vary_header_name.to_string().into(),
    //             variance_value.unwrap_or(VarianceHeaderValue::Multiple(Vec::new())),
    //         );
    //     }

    //     return Some(variance);
    // }

    fn get_req_res_variance(
        req_head: &ReqParts,
        res_head: &ResParts,
    ) -> Option<BTreeMap<HeaderKey, VarianceHeaderValue>> {
        if !res_head.headers.contains_key("Vary") {
            return None;
        }

        let variance_headers = res_head
            .headers
            .get_all("Vary")
            .iter()
            .filter_map(|vary_header| vary_header.to_str().ok())
            .map(|header_str| header_str.to_owned());

        return Self::get_req_variance_headers(&req_head.headers, variance_headers);
    }

    fn get_req_variance_headers<VarianceHeaders>(
        req_headers: &HeaderMap,
        variance_headers: VarianceHeaders,
    ) -> Option<BTreeMap<HeaderKey, VarianceHeaderValue>>
    where
        VarianceHeaders: Iterator<Item: Into<HeaderKey>>,
    {
        let mut variance = BTreeMap::new();

        for header_key in variance_headers {
            let header_key: HeaderKey = header_key.into();

            if variance.contains_key(&header_key) {
                continue;
            }

            let header_key_str: &str = header_key.borrow();
            let vary_values = req_headers.get_all(header_key_str);

            let mut variance_value: Option<VarianceHeaderValue> = None;

            for vary_value in vary_values {
                let vary_value = vary_value.as_bytes().to_owned();

                variance_value = match variance_value {
                    None => Some(VarianceHeaderValue::Single(vary_value)),
                    Some(VarianceHeaderValue::Single(existing)) => {
                        Some(VarianceHeaderValue::Multiple(vec![existing, vary_value]))
                    }
                    Some(VarianceHeaderValue::Multiple(mut existing)) => {
                        existing.push(vary_value);
                        Some(VarianceHeaderValue::Multiple(existing))
                    }
                };
            }

            variance.insert(
                header_key.into(),
                variance_value.unwrap_or(VarianceHeaderValue::Multiple(Vec::new())),
            );
        }

        if variance.len() == 0 {
            return None;
        }

        return Some(variance);
    }

    fn is_vary_processable(res_head: &ResParts) -> bool {
        if res_head
            .headers
            .get_all("Vary")
            .iter()
            .any(|header_value| match header_value.to_str() {
                Err(_) | Ok("*") => true,
                _ => false,
            })
        {
            return false;
        }

        let trailers = res_head.headers.get_all("Vary");

        return !trailers
            .iter()
            .filter_map(|header_value| header_value.to_str().ok())
            .any(|header_value| header_value.eq("Trailer"));
    }
}

impl PartialEq for VarianceHeaderValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Single(l), Self::Single(r)) => l == r,
            (Self::Multiple(l), Self::Multiple(r)) => l == r,
            (Self::Single(l), Self::Multiple(r)) => r.len() == 1 && l == &r[0],
            (Self::Multiple(l), Self::Single(r)) => l.len() == 1 && &l[0] == r,
        }
    }
}

impl Eq for VarianceHeaderValue {}

impl Hash for VarianceHeaderValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            VarianceHeaderValue::Single(value) => value.hash(state),
            VarianceHeaderValue::Multiple(values) if values.len() == 1 => values[0].hash(state),
            VarianceHeaderValue::Multiple(values) if values.len() > 1 => values.hash(state),
            _ => {}
        };
    }
}

impl From<String> for HeaderKey {
    fn from(value: String) -> Self {
        return HeaderKey::String(value);
    }
}

impl From<&'static str> for HeaderKey {
    fn from(value: &'static str) -> Self {
        return HeaderKey::StaticStr(value);
    }
}

impl Borrow<str> for HeaderKey {
    fn borrow(&self) -> &str {
        return match self {
            HeaderKey::String(str) => str.as_str(),
            HeaderKey::StaticStr(str) => str,
        };
    }
}

impl PartialOrd for HeaderKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let this: &str = self.borrow();
        let other: &str = other.borrow();

        return this.partial_cmp(other);
    }
}

impl Ord for HeaderKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let this: &str = self.borrow();
        let other: &str = other.borrow();

        return this.cmp(other);
    }
}

impl PartialEq for HeaderKey {
    fn eq(&self, other: &Self) -> bool {
        let this: &str = self.borrow();
        let other: &str = other.borrow();

        return this.eq(other);
    }
}

impl Eq for HeaderKey {}

impl Hash for HeaderKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let this: &str = self.borrow();
        this.hash(state);
    }
}

use std::{future::Future, net::SocketAddr};

use crate::net::address::Host;

/// An implementation of `DynamicDnsResolver` is used to resolve a hostname to
/// a set of Socket addresses at runtime.
pub trait DynamicDnsResolver: Send + Sync + 'static {
    /// The type of iterator returned by `lookup_host`.
    type Iterator: Iterator<Item = SocketAddr> + Send + 'static;

    /// Resolve the given host with the given port to its set of [`SocketAddr`]es.
    fn lookup_host(
        &self,
        host: Host,
        port: u16,
    ) -> impl Future<Output = Result<Self::Iterator, std::io::Error>> + Send + '_;
}

impl<F, Fut, I> DynamicDnsResolver for F
where
    F: Fn(Host, u16) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<I, std::io::Error>> + Send + 'static,
    I: Iterator<Item = SocketAddr> + Send + 'static,
{
    type Iterator = I;

    fn lookup_host(
        &self,
        host: Host,
        port: u16,
    ) -> impl Future<Output = Result<Self::Iterator, std::io::Error>> + Send + '_ {
        (self)(host, port)
    }
}

impl DynamicDnsResolver for () {
    type Iterator = std::iter::Empty<SocketAddr>;

    async fn lookup_host(&self, _host: Host, _port: u16) -> Result<Self::Iterator, std::io::Error> {
        Ok(std::iter::empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_dns_resolver_contract() {
        fn dynamic_dns_resolver_contract<T: DynamicDnsResolver>(_t: T) {}

        dynamic_dns_resolver_contract(());
        dynamic_dns_resolver_contract(crate::net::lookup_host);
    }
}

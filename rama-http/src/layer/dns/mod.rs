//! Layers in function of DNS.
//!
//! # Example
//!
//! ## [`DnsMapLayer`]
//!
//! Example showing how to allow DNS lookup overwrites
//! using the [`DnsMapLayer`].
//!
//! ```rust
//! use rama_core::{
//!     service::service_fn,
//!     Context, Service, Layer,
//! };
//! use rama_http::{
//!     layer::dns::DnsMapLayer,
//!     HeaderName, Request,
//! };
//! use rama_net::http::RequestContext;
//! use rama_net::address::Host;
//! use std::{
//!     convert::Infallible,
//!     net::{IpAddr, Ipv4Addr},
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     let svc = DnsMapLayer::new(HeaderName::from_static("x-dns-map"))
//!         .layer(service_fn(|mut ctx: Context<()>, req: Request<()>| async move {
//!             match ctx
//!                 .get_or_try_insert_with_ctx::<RequestContext, _>(|ctx| (ctx, &req).try_into())
//!                 .map(|req_ctx| req_ctx.authority.host().clone())
//!             {
//!                 Ok(host) => {
//!                     let domain = match host {
//!                         Host::Name(domain) => domain,
//!                         Host::Address(ip) => panic!("unexpected host: {ip}"),
//!                     };
//!
//!                     let addresses: Vec<_> = ctx
//!                         .dns()
//!                         .ipv4_lookup(domain.clone())
//!                         .await
//!                         .unwrap()
//!                         .collect();
//!                     assert_eq!(addresses, vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))]);
//!
//!                     let addresses: Vec<_> = ctx
//!                         .dns()
//!                         .ipv6_lookup(domain.clone())
//!                         .await
//!                         .unwrap()
//!                         .collect();
//!                     assert!(addresses.is_empty());
//!
//!                     Ok(())
//!                 }
//!                 Err(err) => Err(err),
//!             }
//!         }));
//!
//!     let ctx = Context::default();
//!     let req = Request::builder()
//!         .header("x-dns-map", "example.com=127.0.0.1")
//!         .uri("http://example.com")
//!         .body(())
//!         .unwrap();
//!
//!     svc.serve(ctx, req).await.unwrap();
//! }
//! ```

mod dns_map;
pub use dns_map::{DnsMapLayer, DnsMapService};

mod dns_resolve;
pub use dns_resolve::{
    DnsResolveMode, DnsResolveModeLayer, DnsResolveModeService, DnsResolveModeUsernameParser,
};

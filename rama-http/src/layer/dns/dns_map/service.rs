use std::fmt;

use super::DnsMap;
use crate::{
    layer::header_config::extract_header_config, utils::HeaderValueErr, HeaderName, Request,
};
use rama_core::{error::OpaqueError, Context, Service};
use rama_utils::macros::define_inner_service_accessors;

/// Service to support DNS lookup overwrites.
///
/// No DNS lookup is performed by this service, it only adds
/// the overwrites to the `Dns` (see `rama_core`) data of the [`Context`].
///
/// See [`DnsMapLayer`] for more information.
///
/// [`DnsMapLayer`]: crate::layer::dns::DnsMapLayer
pub struct DnsMapService<S> {
    inner: S,
    header_name: HeaderName,
}

impl<S> DnsMapService<S> {
    /// Create a new instance of the [`DnsMapService`].
    pub const fn new(inner: S, header_name: HeaderName) -> Self {
        Self { inner, header_name }
    }

    define_inner_service_accessors!();
}

impl<S: fmt::Debug> fmt::Debug for DnsMapService<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DnsMapService")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S: Clone> Clone for DnsMapService<S> {
    fn clone(&self) -> Self {
        DnsMapService {
            inner: self.inner.clone(),
            header_name: self.header_name.clone(),
        }
    }
}

impl<State, Body, E, S> Service<State, Request<Body>> for DnsMapService<S>
where
    State: Send + Sync + 'static,
    Body: Send + Sync + 'static,
    E: Into<rama_core::error::BoxError> + Send + Sync + 'static,
    S: Service<State, Request<Body>, Error = E>,
{
    type Response = S::Response;
    type Error = OpaqueError;

    async fn serve(
        &self,
        mut ctx: Context<State>,
        request: Request<Body>,
    ) -> Result<Self::Response, Self::Error> {
        match extract_header_config::<_, DnsMap, _>(&request, &self.header_name) {
            Err(HeaderValueErr::HeaderInvalid(name)) => {
                return Err(OpaqueError::from_display(format!(
                    "Invalid header value for header '{}'",
                    name
                )));
            }
            Err(HeaderValueErr::HeaderMissing(_)) => (), // ignore if missing, it's opt-in
            Ok(dns_map) => {
                ctx.dns_mut().extend_overwrites(dns_map.0)?;
            }
        }

        self.inner
            .serve(ctx, request)
            .await
            .map_err(|err| OpaqueError::from_boxed(err.into()))
    }
}

use crate::boring::dep::boring::{
    pkey::{PKey, Private},
    x509::X509,
};
use crate::types::ApplicationProtocol;

#[derive(Clone, Debug)]
/// Common configuration for a set of server sessions.
pub struct ServerConfig {
    /// Private Key of the server
    pub private_key: PKey<Private>,
    /// CA Cert Chain of the server
    pub ca_cert_chain: Vec<X509>,
    /// Set the ALPN protocols supported by the service's inner application service.
    pub alpn_protocols: Vec<ApplicationProtocol>,
    /// Write logging information to facilitate tls interception.
    pub keylog_filename: Option<String>,
}

impl ServerConfig {
    /// Create a new [`ServerConfig`].
    pub const fn new(private_key: PKey<Private>, ca_cert_chain: Vec<X509>) -> ServerConfig {
        ServerConfig {
            private_key,
            ca_cert_chain,
            alpn_protocols: vec![],
            keylog_filename: None,
        }
    }
}

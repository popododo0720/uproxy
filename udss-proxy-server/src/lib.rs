pub mod proxy_server;
mod proxy_handler;
mod http;
mod https;

pub use proxy_server::ProxyServer;
pub use http::{handle_http_request, convert_relative_to_absolute_uri, create_error_response};
pub use https::{handle_https_request};
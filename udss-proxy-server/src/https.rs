use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use log::{debug, error};
use std::sync::Arc;

use udss_proxy_error::{ProxyError, Result};

/// HTTP 요청 업스트림 포워딩
pub async fn handle_https_request(
    req: Request<Incoming>,
    client: Arc<HyperClient<HttpConnector, Full<Bytes>>>,
) -> Result<Response<Full<Bytes>>> {
    Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))
            .unwrap())
}

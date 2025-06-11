use http_body_util::{Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use log::{debug, info};
use std::sync::Arc;

use crate::http::{handle_http_request, create_error_response};
use crate::https::{handle_https_request};

use udss_proxy_acl::domain_blocker::DomainBlocker;
use udss_proxy_error::{Result};

/// 프록시 요청 핸들러
pub async fn proxy_handler(
    req: Request<Incoming>,
    client: Arc<HyperClient<HttpConnector, Full<Bytes>>>,
    blocker: Arc<DomainBlocker>,
) -> Result<Response<Full<Bytes>>> {
    debug!("incoming: {req:?}");

    // 직접 프록시 서버로 보내는 요청에 대한 기본 응답 (모든 경로 차단)
    if req.uri().authority().is_none() { // path가 무엇이든 상관없이 authority가 없는 모든 요청 차단
        debug!("직접 요청 감지: URI={}", req.uri());
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(Full::new(Bytes::from("This is a proxy server. Direct requests are not allowed.")))
            .unwrap());
    }

    // 요청 URI에서 호스트 정보 추출 및 차단 여부 확인
    if let Some(host_str) = req.uri().host() {
        if !host_str.is_empty() && blocker.is_blocked(host_str) {
            info!("차단된 도메인 요청: {} (Host: {})", req.uri(), host_str);
            return Ok(create_error_response(
                StatusCode::FORBIDDEN,
                &format!("Access to the domain '{host_str}' is blocked by policy."),
            ));
        }
    } else {
        debug!("요청 URI에 host 정보 없음: {}", req.uri());
    }

    // https
    if Method::CONNECT == req.method() {
        handle_https_request(req, client).await
    } else {
        // http
        handle_http_request(req, client).await
    }
}



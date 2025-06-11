use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use log::{debug, error};
use std::sync::Arc;

use udss_proxy_error::{ProxyError, Result};

use crate::state::AppState;

/// HTTP 요청 업스트림 포워딩
pub async fn handle_http_request(
    req: Request<Incoming>,
    state: AppState,
) -> Result<Response<Full<Bytes>>> {
    let (mut parts, body) = req.into_parts();

    let body_bytes = body.collect().await?.to_bytes();
    let outgoing_req = Request::from_parts(parts, Full::new(body_bytes));
    debug!("서버로 요청 포워딩: {}", outgoing_req.uri());

    // 요청 스킴에 따라 올바른 클라이언트 선택
    let response_result = if outgoing_req.uri().scheme_str() == Some("https") {
        state.https_client.request(outgoing_req).await
    } else {
        state.http_client.request(outgoing_req).await
    };

    match response_result {
        Ok(response) => {
            debug!("응답코드: {}", response.status());
            let (parts, body) = response.into_parts();
            let body_bytes = body.collect().await?.to_bytes();
            Ok(Response::from_parts(parts, Full::new(body_bytes)))
        }
        Err(e) => {
            error!("업스트림 요청 실패: {e}");
            Ok(create_error_response(
                StatusCode::BAD_GATEWAY,
                "Upstream request failed",
            ))
        }
    }
}

/// 상대 URI 절대 URI로 변환
pub fn convert_relative_to_absolute_uri(
    parts: &mut hyper::http::request::Parts,
    is_tls: bool,
) -> Result<()> {
    if let Some(host_header) = parts
        .headers
        .get(hyper::header::HOST)
        .and_then(|h| h.to_str().ok())
    {
        let scheme = if is_tls { "https" } else { "http" };

        let new_uri_str = format!(
            "{}://{}{}",
            scheme,
            host_header,
            parts
                .uri
                .path_and_query()
                .map_or("", hyper::http::uri::PathAndQuery::as_str)
        );

        // URI 파싱 및 설정
        match new_uri_str.parse::<hyper::Uri>() {
            Ok(new_uri) => {
                parts.uri = new_uri;
                Ok(())
            }
            Err(e) => {
                error!("uri 변환 실패 '{new_uri_str}' : {e}");
                Err(ProxyError::Http(format!("Invalid URI format: {e}")))
            }
        }
    } else {
        error!("상대 URI에 대한 호스트 헤더 누락: {}", parts.uri);
        Err(ProxyError::Http(
            "Host header required for relative URI".to_string(),
        ))
    }
}

/// 에러응답
pub fn create_error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .unwrap()
}

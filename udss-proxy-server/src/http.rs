use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use log::{debug, error};
use std::sync::Arc;

use udss_proxy_error::{ProxyError, Result};

/// HTTP 요청 업스트림 포워딩
pub async fn handle_http_request(
    req: Request<Incoming>,
    client: Arc<HyperClient<HttpConnector, Full<Bytes>>>,
) -> Result<Response<Full<Bytes>>> {
    let (mut parts, body) = req.into_parts();

    // URI 변환
    if parts.uri.scheme().is_none() {
        convert_relative_to_absolute_uri(&mut parts, false)?;
    }

    // 요청 바디를 Full<Bytes>로 변환
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!("요청 바디 읽기 실패: {e}");
            return Ok(create_error_response(
                StatusCode::BAD_REQUEST,
                "Failed to read request body",
            ));
        }
    };

    let outgoing_req = Request::from_parts(parts, Full::new(body_bytes));
    debug!("서버로 요청 포워딩: {}", outgoing_req.uri());

    // 업스트림으로 요청 전송
    match client.request(outgoing_req).await {
        Ok(response) => {
            debug!("응답코드: {}", response.status());

            let (parts, body) = response.into_parts();
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    error!("응답 바디 읽기 실패: {e}");
                    return Ok(create_error_response(
                        StatusCode::BAD_GATEWAY,
                        "Failed to read response body",
                    ));
                }
            };

            Ok(Response::from_parts(parts, Full::new(body_bytes)))
        }
        Err(e) => {
            error!("요청 포워딩 실패: {e}");
            Ok(create_error_response(
                StatusCode::BAD_GATEWAY,
                "Failed to connect to upstream",
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

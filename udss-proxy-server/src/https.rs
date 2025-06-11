use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper::service::service_fn;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioIo;
use log::{debug, error};
use rustls::{ServerConfig};
use tokio_rustls::TlsAcceptor;
use udss_proxy_error::{ProxyError, Result};

use crate::state::AppState;
use crate::http::handle_http_request;

/// HTTP 요청 업스트림 포워딩
pub async fn handle_https_request(
    req: Request<Incoming>,
    state: AppState,
) -> Result<Response<Full<Bytes>>> {
    let host = req.uri().host().unwrap_or_default().to_string();
    if host.is_empty() {
        return Ok(create_error_response(StatusCode::BAD_REQUEST, "CONNECT 요청에 호스트가 없습니다."));
    }

    tokio::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                if let Err(e) = serve_tls_connection(upgraded, state, &host).await {
                    error!("TLS 연결 처리 실패 [{}]: {}", host, e);
                }
            }
            Err(e) => error!("HTTPS 업그레이드 실패: {}", e),
        }
    });

    // 클라이언트에게 연결이 수립되었음을 알립니다.
    Ok(Response::new(Full::new(Bytes::new())))
}

/// 업그레이드 된 커넥션에 tls 적용 및 복호화 요청처리
async fn serve_tls_connection(
    upgraded: hyper::upgrade::Upgraded,
    state: AppState,
    host: &str,
) -> Result<()> {
    // TODO: udss-proxy-tls 크레이트에서 루트 CA와 키를 로드해야 합니다.
    // let (root_ca, root_key) = get_root_ca()?;
    // let server_cert = generate_server_cert(host, &root_ca, &root_key)?;

    // 이 부분은 실제 인증서 생성 로직으로 대체되어야 합니다.
    // 아래는 임시 코드입니다.
    let (cert_chain, private_key) = udss_proxy_tls::certs::generate_temp_cert_for_host(host)?;
    
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .map_err(|e| ProxyError::Tls(format!("Rustls 설정 실패: {}", e)))?;

    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let stream = acceptor.accept(upgraded).await?;
    let io = TokioIo::new(stream);

    // 복호화된 스트림 위에서 다시 HTTP 서비스를 실행합니다.
    let service = service_fn(move |req| {
        let state_clone = state.clone();
        // 복호화된 요청은 일반 HTTP 요청처럼 처리합니다.
        handle_http_request(req, state_clone)
    });

    http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(io, service)
        .with_upgrades()
        .await
        .map_err(|e| ProxyError::Http(format!("복호화된 연결 서빙 실패: {}", e)))?;

    Ok(())
}

// 임시 에러 응답 생성 함수
fn create_error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(message.to_owned())))
        .unwrap()
}
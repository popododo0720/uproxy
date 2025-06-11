use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoConnBuilder;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::Duration;

use udss_proxy_acl::domain_blocker::DomainBlocker;
use udss_proxy_config::setting::Settings;
use udss_proxy_error::{ProxyError, Result};

/// 프록시 서버 구조체
pub struct ProxyServer {
    /// 서버 설정 정보
    setting: Settings,
    /// HTTP 클라이언트 연결 풀
    client_pool: Arc<HyperClient<HttpConnector, Full<Bytes>>>,
    /// 도메인 차단기
    domain_blocker: Arc<DomainBlocker>,
}

impl ProxyServer {
    /// 새로운 프록시 서버 인스턴스를 생성
    pub fn new(setting: Settings, domain_blocker: Arc<DomainBlocker>) -> Self {
        // HTTP 커넥터 설정
        let mut connector = HttpConnector::new();
        connector.set_keepalive(Some(Duration::from_secs(30))); // 연결 유지 시간
        connector.set_nodelay(true); // TCP_NODELAY 활성화 (지연 최소화)
        connector.set_reuse_address(true); // 주소 재사용 허용

        // HTTP 클라이언트 생성 (연결 풀링 설정)
        let client = Arc::new(
            HyperClient::builder(TokioExecutor::default())
                .pool_idle_timeout(Duration::from_secs(30)) // 유휴 연결 타임아웃
                .pool_max_idle_per_host(100) // 호스트당 최대 유휴 연결 수
                .build(connector),
        );

        Self {
            setting,
            client_pool: client,
            domain_blocker,
        }
    }

    /// 서버실행
    pub async fn run(&self) -> Result<()> {
        // 바인딩 주소
        let addr = format!(
            "{}:{}",
            self.setting.proxy.bind_host, self.setting.proxy.bind_port
        );
        let listener = TcpListener::bind(&addr).await?;
        info!("프록시 서버 시작: {addr}");

        let client = self.client_pool.clone();
        let domain_blocker = self.domain_blocker.clone();

        loop {
            let (stream, client_addr) = listener.accept().await?;
            let client_clone = client.clone();
            let blocker_clone = domain_blocker.clone();
            let self_host = self.setting.proxy.bind_host.clone();
            let self_port = self.setting.proxy.bind_port;

            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                if let Err(err) = AutoConnBuilder::new(TokioExecutor::default())
                    .serve_connection(
                        io,
                        service_fn(move |req| {
                            proxy_handler(req, client_clone.clone(), blocker_clone.clone(), self_host.clone(), self_port)
                        }),
                    )
                    .await
                {
                    error!("커넥션 에러: {err}");
                } else {
                    debug!("커넥션 종료: {client_addr}");
                }
            });
        }
    }
}

/// 프록시 요청 핸들러
async fn proxy_handler(
    req: Request<Incoming>,
    client: Arc<HyperClient<HttpConnector, Full<Bytes>>>,
    blocker: Arc<DomainBlocker>,
    self_host: String,
    self_port: u16,
) -> Result<Response<Full<Bytes>>> {
    debug!("incoming: {req:?}");

    // 자신으로의 요청 차단
    if is_self_request(&req, &self_host, self_port) {
        return Ok(create_error_response(
            StatusCode::LOOP_DETECTED,
            "Request to self detected - preventing infinite loop"
        ));
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

    // CONNECT 메서드 처리 (HTTPS 터널링)
    if Method::CONNECT == req.method() {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))
            .unwrap())
    } else {
        // 일반 HTTP 요청 처리
        handle_http_request(req, client).await
    }
}

/// 자신으로의 요청인지 확인
fn is_self_request(req: &Request<Incoming>, self_host: &String, self_port: u16) -> bool {
    let uri = req.uri();

    // URI에서 호스트와 포트 추출
    if let Some(authority) = uri.authority() {
        let target_host = authority.host();
        let target_port = authority.port_u16().unwrap_or(80);
        
        // 자기 자신의 주소와 비교
        // 호스트 비교 (localhost, 127.0.0.1, 0.0.0.0 등 고려)
        let is_same_host = target_host == self_host ||
            (is_localhost(target_host) && is_localhost(self_host)) ||
            (target_host == "0.0.0.0" || self_host == "0.0.0.0");
        
        return is_same_host && target_port == self_port;
    }

    false
}

/// localhost 계열 주소인지 확인
fn is_localhost(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

/// HTTP 요청 업스트림 포워딩
async fn handle_http_request(
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
fn convert_relative_to_absolute_uri(
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
fn create_error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .unwrap()
}

use http_body_util::{Full};
use hyper::body::{Bytes};
use hyper::service::service_fn;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoConnBuilder;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::Duration;

use crate::proxy_handler::proxy_handler;

use udss_proxy_acl::domain_blocker::DomainBlocker;
use udss_proxy_config::setting::Settings;
use udss_proxy_error::{Result};

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

            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                if let Err(err) = AutoConnBuilder::new(TokioExecutor::default())
                    .serve_connection(
                        io,
                        service_fn(move |req| {
                            proxy_handler(req, client_clone.clone(), blocker_clone.clone())
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



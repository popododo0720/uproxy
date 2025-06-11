use std::io::Write;
use std::sync::Arc;
use tokio::time::Duration;

use chrono::Local;
use env_logger::Builder;
use log::{LevelFilter, info, warn};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;


use udss_proxy_acl::domain_blocker::DomainBlocker;
use udss_proxy_config::Settings;
use udss_proxy_db::{initialize_db, initialize_dbpool};
use udss_proxy_error::Result;
use udss_proxy_server::state::AppState;
use udss_proxy_tls::certs::{ensure_ssl_directories, init_root_ca, load_trusted_certificates};

#[tokio::main]
async fn main() -> Result<()> {
    // fd 세팅
    setup_resource_limits();

    // 로거 세팅
    setup_logger();

    info!("udss-proxy 서버 시작");

    // 통합 설정 로드
    let mut settings = Settings::new()?;

    // SSL 디렉토리 확인 및 생성
    ensure_ssl_directories(&settings.proxy)?;

    // 신뢰할 인증서 로드
    load_trusted_certificates(&mut settings.proxy)?;

    // tls 루트 ca 인증서 초기화
    init_root_ca(&settings.proxy).await?;

    // db 초기화
    let db_pool = initialize_dbpool(&settings.database).await?;
    initialize_db(&settings.database, &db_pool).await?;

    // 도메인 차단기 초기화
    let domain_blocker = Arc::new(DomainBlocker::new());
    domain_blocker.init(&db_pool).await?;

    // 1. 일반 HTTP 통신을 위한 클라이언트
    let http_client = Arc::new(
        HyperClient::builder(TokioExecutor::default()).build(HttpConnector::new()),
    );

    // 2. HTTPS 통신을 위한 클라이언트
    let https_connector = HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http1()
        .build();

    let https_client = Arc::new(
        HyperClient::builder(TokioExecutor::default()).build(https_connector),
    );

    // AppState 생성
    let app_state = AppState {
        http_client, 
        https_client,
        settings: Arc::new(settings),
        blocker: domain_blocker,
        db_pool,
    };

    // 서버시작
    run(app_state).await?;

    Ok(())
}

/// 파일 디스크립터 제한 설정
static FD_LIMIT: std::sync::LazyLock<u64> = std::sync::LazyLock::new(|| {
    std::env::var("FD_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000) // 기본값 100k
});

/// 시스템 리소스 제한 설정
fn setup_resource_limits() {
    #[cfg(unix)]
    {
        use nix::sys::resource::{Resource, setrlimit};
        // fd 제한 늘리기
        match setrlimit(Resource::RLIMIT_NOFILE, *FD_LIMIT, *FD_LIMIT) {
            Ok(_) => {
                info!("파일 디스크립터 제한 {}", *FD_LIMIT);
            }
            Err(e) => {
                warn!("파일 디스크립터 제한 설정 실패: {e:?}");
            }
        }
    }
}

/// 로거 세팅
fn setup_logger() {
    #[cfg(debug_assertions)]
    {
        Builder::new()
            .filter(None, LevelFilter::Debug)
            .format(|buf, record| {
                writeln!(
                    buf,
                    "[{} {} {}:{}] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.args()
                )
            })
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        Builder::new()
            .filter(None, LevelFilter::Info)
            .format(|buf, record| {
                writeln!(
                    buf,
                    "[{} {} {}:{}] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.args()
                )
            })
            .init();
    }
}

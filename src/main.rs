use std::io::Write;




use once_cell::sync::Lazy;
use log::{info, warn, LevelFilter};
use env_logger::Builder;
use chrono::Local;

use udss_proxy_config::Settings;
use udss_proxy_error::{Result};
use udss_proxy_tls::certs::{init_root_ca, ensure_ssl_directories, load_trusted_certificates};
use udss_proxy_db::initialize_database;

/// 파일 디스크립터 제한 설정
static FD_LIMIT: Lazy<u64> = Lazy::new(|| {
    std::env::var("FD_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000000) // 기본값 1M
});

/// 시스템 리소스 제한 설정
fn setup_resource_limits() {
    #[cfg(unix)]
    {
        use nix::sys::resource::{setrlimit, Resource};
        // fd 제한 늘리기
        match setrlimit(Resource::RLIMIT_NOFILE, *FD_LIMIT, *FD_LIMIT) {
            Ok(_) => {
                info!("파일 디스크립터 제한 {}", *FD_LIMIT);
            },
            Err(e) => {
                warn!("파일 디스크립터 제한 설정 실패: {:?}", e);
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
            .format(|buf,record| {
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
            .init()
    }
    
    #[cfg(not(debug_assertions))]
    {
        Builder::new()
            .filter(None, LevelFilter::Info)
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<()>{
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

    // db 세팅
    let db_pool = initialize_database(&settings.database).await?;
    

    Ok(())
}
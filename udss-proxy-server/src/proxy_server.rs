use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use log::{debug, error, info};
use tokio::net::TcpListener;

use crate::proxy_handler::proxy_handler;
use crate::state::AppState;

use udss_proxy_error::Result;

/// 서버 실행
pub async fn run(state: AppState) -> Result<()> {
    // 바인딩 주소
    let addr = format!(
        "{}:{}",
        state.settings.proxy.bind_host, state.settings.proxy.bind_port
    );
    let listener = TcpListener::bind(&addr).await?;
    info!("프록시 서버 시작: {}", addr);

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let state_clone = state.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| proxy_handler(req, state_clone.clone()));

            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                error!("커넥션 처리 중 오류 발생: {}", err);
            }
            debug!("커넥션 종료: {}", client_addr);
        });
    }
}
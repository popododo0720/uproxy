use std::sync::{Arc};

use hyper_util::client::legacy::connect::HttpConnector;
use tokio::net::{TcpListener};
use tokio::sync::{mpsc};
use log::{debug, info, error};
use hyper::service::{service_fn};
use hyper::body::{Bytes, Incoming};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::server::conn::auto::Builder as AutoConnBuilder;
use hyper_util::rt::{TokioIo, TokioExecutor};
use hyper_util::client::legacy::Client as HyperClient;
use http_body_util::{Full, BodyExt};

use udss_proxy_config::setting::Settings;
use udss_proxy_error::{Result, ProxyError};

pub struct ProxyServer {
    setting: Settings,
}

impl ProxyServer {
    pub fn new(setting: Settings) -> Self {
        Self {
            setting,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let addr = format!("{}:{}", self.setting.proxy.bind_host, self.setting.proxy.bind_port);
        let listener = TcpListener::bind(&addr).await?;
        info!("프록시 서버 시작: {}", addr);

        // 워커 스레드 설정
        let worker_threads = self.setting.proxy.worker_threads.unwrap_or_else(|| num_cpus::get()*4);
        info!("워커 스레드 수: {}", worker_threads);

        let (tx, rx) = mpsc::channel(2000);
        let rx = Arc::new(tokio::sync::Mutex::new(rx));

        // 워커스레드 생성
        for worker_id in 0..worker_threads {
            let worker_rx = rx.clone();
            let _worker_config = self.setting.proxy.clone();

            tokio::spawn(async move {
                debug!("워커 스레드 {} 시작", worker_id);

                loop {
                    let (client_stream, client_addr) = {
                        let mut rx_guard = worker_rx.lock().await;
                        match rx_guard.recv().await {
                            Some(conn) => conn,
                            None => {
                                error!("워커 {} 종료", worker_id);
                                break;
                            }
                        }
                    };

                    let io = TokioIo::new(client_stream);

                    tokio::spawn(async move {
                        if let Err(err) = AutoConnBuilder::new(TokioExecutor::default())
                            .serve_connection(io, service_fn(proxy_handler))
                            .await 
                        {
                            error!("커넥션 에러: {}", err);
                        } else {
                            info!("커넥션 종료: {}", client_addr);
                        }
                    });
                }
            });
        }

        // 메인루프
        loop {
            let (stream, client_addr) = listener.accept().await?;
            debug!("새 커넥션: {}", client_addr);

            if let Err(_) = tx.send((stream, client_addr)).await {
                error!("워커 채널이 닫혔습니다.");
                break;
            }
        }

        Ok(())
    }
}

async fn proxy_handler(req: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    debug!("incoming: {:?}", req);

    if Method::CONNECT == req.method() {
        Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::new()))
        .unwrap())
    } else {
        handle_http_request(req).await
    }
}
    
async fn handle_http_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    let client_connector = HttpConnector::new();
    let client = HyperClient::builder(TokioExecutor::default()).build(client_connector);

    let (mut parts, body) = req.into_parts();

    // uri 변환
    if parts.uri.scheme().is_none() {
        if let Err(e) = convert_relative_to_absolute_uri(&mut parts) {
            return Err(e);
        }
    }

    let outgoing_req = Request::from_parts(parts, body);
    debug!("서버로 요청 포워딩: {}", outgoing_req.uri());

    //upstreams
    match client.request(outgoing_req).await {
        Ok(response) => {
            debug!("응답코드: {}", response.status());
            convert_response_body(response).await
        }
        Err(e) => {
            error!("요청 포워딩 실패: {}", e);
            Ok(create_error_response(StatusCode::BAD_GATEWAY, "Failed to connect to upstream"))
        }
    }
}

fn convert_relative_to_absolute_uri(parts: &mut hyper::http::request::Parts) -> Result<()> {
    if let Some(host_header) = parts.headers.get(hyper::header::HOST)
        .and_then(|h| h.to_str().ok()) 
    {
        let scheme = if parts.uri.port_u16() == Some(443) { "https" } else { "http" };
        let new_uri_str = format!(
        "{}://{}{}",
        scheme,
        host_header,
        parts.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("")
        );
        
        match new_uri_str.parse::<hyper::Uri>() {
            Ok(new_uri) => {
                parts.uri = new_uri;
                Ok(())
            }
            Err(e) => {
                error!("uri 변환 실패 '{}' : {}", new_uri_str, e);
                Err(ProxyError::Http(format!("Invalid URI format: {}", e)))
            }
        }
    } else {
        error!("상대 URI에 대한 호스트 헤더 누락: {}", parts.uri);
        Err(ProxyError::Http("Host header required for relative URI".to_string()))
    }
}

async fn convert_response_body(response: Response<Incoming>) -> Result<Response<Full<Bytes>>> {
    let (parts, body) = response.into_parts();

    match body.collect().await {
        Ok(collected) => Ok(Response::from_parts(parts, Full::new(collected.to_bytes()))),
        Err(e) => {
            error!("응답 바디 읽기 실패: {}", e);
            Err(ProxyError::Http(format!("Failed to read upstream response body: {}", e)))
        }
    }
}

// 에러응답
fn create_error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .unwrap()
}
// udss-proxy-server/src/state.rs

use std::sync::Arc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;

use udss_proxy_acl::domain_blocker::DomainBlocker;
use udss_proxy_config::setting::Settings;
use udss_proxy_db::pool::DatabasePool;

/// 애플리케이션의 공유 상태를 관리하는 구조체
#[derive(Clone)]
pub struct AppState {
    pub http_client: Arc<HyperClient<HttpConnector, Full<Bytes>>>,
    pub https_client: Arc<HyperClient<hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>>,
    pub settings: Arc<Settings>,
    pub blocker: Arc<DomainBlocker>,
    pub db_pool: DatabasePool,
}
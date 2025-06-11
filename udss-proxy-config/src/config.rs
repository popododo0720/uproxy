use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use udss_proxy_error::Result;

/// 프록시 서버 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind_host: String,
    pub bind_port: u16,
    pub buffer_size: usize,
    pub timeout_ms: usize,
    pub ssl_dir: String,
    pub worker_threads: Option<usize>,
    pub tls_verify_certificate: bool,
    pub disable_verify_internal_ip: bool,
    pub trusted_certificates: Vec<String>,
    pub cache_enabled: bool,
    pub cache_size: usize,
    pub cache_ttl_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// 기본설정으로 생성
    #[must_use]
    pub fn new() -> Self {
        Self {
            bind_host: "0.0.0.0".to_string(),
            bind_port: 50000,
            buffer_size: 16384,
            timeout_ms: 60000,
            ssl_dir: "ssl".to_string(),
            worker_threads: None,
            tls_verify_certificate: true,
            disable_verify_internal_ip: false,
            trusted_certificates: Vec::new(),
            cache_enabled: true,
            cache_size: 1000,
            cache_ttl_seconds: 300,
        }
    }

    /// 설정파일에서 설정 로드
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let config = serde_yml::from_str(&contents)?;

        Ok(config)
    }
}

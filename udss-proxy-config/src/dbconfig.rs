use std::{path::Path};
use std::fs::File;
use std::io::Read;

use serde::{Deserialize, Serialize};

use udss_proxy_error::{Result};

/// 데이터베이스 설정
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    /// 데이터베이스 연결 설정
    pub connection: ConnectionConfig,
    /// 파티셔닝 설정
    pub partitioning: PartitionConfig,
    /// 연결 풀 설정
    pub pool: PoolConfig,
}

impl DbConfig {
    /// 설정파일에서 db 설정 로드
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let config: DbConfig = serde_yml::from_str(&contents)?;

        Ok(config)
    }
}

/// db 연결설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub sslmode: String,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "alicedb".to_string(),
            user: "dbadmin".to_string(),
            password: "dbadminpass".to_string(),
            sslmode: "disable".to_string(),
        }
    }
}

/// 데이터베이스 파티셔닝 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionConfig {
    /// 파티션 생성 간격
    pub creation_interval: u32,
    /// 파티션 보존 기간
    pub retention_period: u32,
    /// 미래 파티션 수
    pub future_partitions: u32,
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self {
            creation_interval: 1,
            retention_period: 365,
            future_partitions: 1,
        }
    }
}

/// 데이터베이스 연결 풀 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 최대 연결 수
    pub max_connections: usize,
    /// 연결 타임아웃(초)
    pub connection_timeout_seconds: u64,
    /// 연결 재사용 전 대기 시간(초)
    pub recycle_seconds: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 20, // 최대 연결풀 
            connection_timeout_seconds: 30,  // 연결 시도 타임아웃 30초
            recycle_seconds: 21_600,    // 6시간마다 연결 갱신
        }
    }
}


use std::sync::Arc;
use std::time::Duration;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use log::info;
use tokio_postgres::{
    NoTls,
    config::{Config, SslMode},
};

use udss_proxy_config::DbConfig;
use udss_proxy_error::{ProxyError, Result};

/// db 풀 인스턴스
#[derive(Clone)]
pub struct DatabasePool {
    pool: Arc<Pool>,
}

impl DatabasePool {
    /// db 풀 생성
    pub async fn new(dbconfig: &DbConfig) -> Result<Self> {
        info!("db 풀 초기화");

        // PostgreSQL 설정 생성
        let pg_config = Self::create_pg_config(dbconfig);

        // 연결 풀 생성
        let pool = Self::create_connection_pool(pg_config, dbconfig).await?;

        info!(
            "데이터베이스 연결 풀 초기화 완료 (최대 연결 수: {})",
            dbconfig.pool.max_connections
        );

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// `PostgreSQL` 설정 생성
    fn create_pg_config(dbconfig: &DbConfig) -> Config {
        let ssl_mode = match dbconfig.connection.sslmode.to_lowercase().as_str() {
            "disable" => SslMode::Disable,
            "require" => SslMode::Require,
            _ => SslMode::Prefer,
        };

        let mut pg_config = Config::new();
        pg_config
            .host(dbconfig.connection.host.as_str())
            .port(dbconfig.connection.port)
            .dbname(dbconfig.connection.database.as_str())
            .user(dbconfig.connection.user.as_str())
            .password(dbconfig.connection.password.as_str())
            .ssl_mode(ssl_mode)
            .connect_timeout(Duration::from_secs(
                dbconfig.pool.connection_timeout_seconds,
            ))
            .keepalives(true);

        pg_config
    }

    /// 연결 풀 생성 및 테스트
    async fn create_connection_pool(pg_config: Config, dbconfig: &DbConfig) -> Result<Pool> {
        // 연결 풀 설정
        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);

        // 풀 빌더 설정
        let pool = Pool::builder(mgr)
            .max_size(dbconfig.pool.max_connections)
            .runtime(Runtime::Tokio1)
            .recycle_timeout(Some(Duration::from_secs(dbconfig.pool.recycle_seconds)))
            .build()
            .map_err(|e| ProxyError::Database(format!("db 풀 생성 실패: {e}")))?;

        // 연결 테스트
        let conn = pool
            .get()
            .await
            .map_err(|e| ProxyError::Database(format!("데이터베이스 연결 테스트 실패: {e}")))?;

        // 간단한 쿼리로 연결 확인
        conn.query_one("SELECT 1", &[])
            .await
            .map_err(|e| ProxyError::Database(format!("데이터베이스 쿼리 테스트 실패: {e}")))?;

        Ok(pool)
    }

    /// 연결 풀에서 연결 가져오기
    pub async fn get_connection(&self) -> Result<deadpool_postgres::Object> {
        self.pool
            .get()
            .await
            .map_err(|e| ProxyError::Database(format!("연결 풀에서 연결 가져오기 실패: {e}")))
    }

    /// 연결 풀 상태 정보
    pub fn pool_status(&self) -> PoolStatus {
        let status = self.pool.status();
        PoolStatus {
            size: status.size,
            available: status.available,
            waiting: status.waiting,
        }
    }
}

/// 연결 풀 상태 정보
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
}

/// 데이터베이스 풀 초기화 함수
pub async fn initialize_dbpool(config: &DbConfig) -> Result<DatabasePool> {
    DatabasePool::new(config).await
}

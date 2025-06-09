use log::{debug, warn};

use udss_proxy_config::DbConfig;
use udss_proxy_error::{Result};

use crate::{partition::PartitionManager};
use crate::{pool::DatabasePool};

/// 데이터베이스 초기화
pub async fn initialize_db(config: &DbConfig, pool: &DatabasePool) -> Result<()> {
    debug!("데이터베이스 파티션 확인");
    match ensure_partitions(config.clone(), pool).await {
        Ok(_) => debug!("데이터베이스 파티션 확인 완료"),
        Err(e) => warn!("데이터베이스 파티션 확인 실패: {:?}", e),
    }
    Ok(())
}

/// 파티션 상태 확인
async fn ensure_partitions(config: DbConfig, pool: &DatabasePool) -> Result<()> {
    // 파티션매니저 생성
    let partition_manager = PartitionManager::new(config);

    // 테이블 생성 확인
    let client = match get_client().await {
        Ok(client) => client,
        Err(e) => {
            error!("DB 연결 실패: {}", e);
            return Err(Box::new(e));
        }
    };
    Ok(())
}
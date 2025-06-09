

use udss_proxy_config::DbConfig;
use udss_proxy_error::{Result};


/// 파티션 관리자
pub struct PartitionManager {
    config: DbConfig,
}

impl PartitionManager {
    /// 새 PartitionManager 생성
    pub fn new(config: DbConfig) -> Self {
        Self {
            config,
        }
    }

    /// 파티션생성
    async fn create_partitions(&self) -> Result<()> {
        Ok(())
    }
}
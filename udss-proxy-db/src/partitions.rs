use log::{error, info};

use udss_proxy_error::Result;

/// 테이블 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableType {
    RequestLogs,
    ResponseLogs,
    ProxyStats,
    ProxyStatsHourly,
}

impl TableType {
    /// 테이블 이름 반환
    const fn get_name(self) -> &'static str {
        match self {
            TableType::RequestLogs => "request_logs",
            TableType::ResponseLogs => "response_logs",
            TableType::ProxyStats => "proxy_stats",
            TableType::ProxyStatsHourly => "proxy_stats_hourly",
        }
    }
}

/// 파티션 생성
pub async fn create_partitions(
    conn: &deadpool_postgres::Object,
    table: TableType,
    num_days: i32,
) -> Result<()> {
    // let mut created_partitions = Vec::new();
    let table_name = table.get_name();

    // 템플릿에 값을 직접 삽입하여 SQL 생성
    let sql = format!(
        include_str!("./sql/create_future_partitions.sql"),
        table_name, num_days
    );

    // 저장된 SQL 스크립트 사용
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(10), // 10초 타임아웃
        conn.execute(&sql, &[]),
    )
    .await
    {
        Ok(Ok(_)) => {
            info!("파티션 자동 생성 스크립트 실행 완료: {table_name}, {num_days} 일 생성");
        }
        Ok(Err(e)) => {
            error!("파티션 생성 스크립트 실행 실패: {e}");
            return Err(e.into());
        }
        Err(_) => {
            error!("파티션 생성 스크립트 실행 타임아웃");
            return Err("파티션 생성 스크립트 실행 타임아웃".into());
        }
    }

    Ok(())
}


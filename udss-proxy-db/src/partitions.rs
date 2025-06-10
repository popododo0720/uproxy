use log::{info, error};
use chrono::{Duration, Datelike};

use udss_proxy_error::{Result};

/// 테이블 유형
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableType {
    RequestLogs,
    ResponseLogs,
    ProxyStats,
    ProxyStatsHourly,
}

impl TableType {
    /// 테이블 이름 반환
    fn get_name(&self) -> &'static str {
        match self {
            TableType::RequestLogs => "request_logs",
            TableType::ResponseLogs => "response_logs",
            TableType::ProxyStats => "proxy_stats",
            TableType::ProxyStatsHourly => "proxy_stats_hourly",
        }
    }
}

/// 파티션 생성
pub async fn create_partitions(conn: &deadpool_postgres::Object, table: TableType, start_date: chrono::NaiveDate, num_days: i32) -> Result<()> {
    let mut created_partitions = Vec::new();
    let table_name = table.get_name();

    // 템플릿에 값을 직접 삽입하여 SQL 생성
    let sql = format!(
        include_str!("./sql/create_future_partitions.sql"),
        table_name, num_days
    );

    // 저장된 SQL 스크립트 사용
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(10), // 10초 타임아웃
        conn.execute(&sql, &[])
    ).await {
        Ok(Ok(_)) => {
            info!("파티션 자동 생성 스크립트 실행 완료: {}, {} 일 생성", table_name, num_days);
            
            // 생성된 파티션 이름 구성
            for i in 0..num_days {
                let date = start_date + Duration::days(i as i64);
                let partition_name = get_partition_name(table_name, date);
                created_partitions.push(partition_name);
            }
        },
        Ok(Err(e)) => {
            error!("파티션 생성 스크립트 실행 실패: {}", e);
            return Err(e.into());
        },
        Err(_) => {
            error!("파티션 생성 스크립트 실행 타임아웃");
            return Err("파티션 생성 스크립트 실행 타임아웃".into());
        }
    }

    Ok(())
}

/// 파티션 이름 생성
fn get_partition_name(table_name: &str, date: chrono::NaiveDate) -> String {
    let year = date.year();
    let month = date.month();
    let day = date.day();
    
    format!("{}_{:04}{:02}{:02}", 
        table_name, 
        year, 
        month, 
        day
    )
}
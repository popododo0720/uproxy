use log::{debug, error, info, warn};

use udss_proxy_config::DbConfig;
use udss_proxy_error::Result;

use crate::partitions::{TableType, create_partitions};
use crate::pool::DatabasePool;
use crate::sql::{
    domain_blocks, domain_pattern_blocks, proxy_stats, proxy_stats_hourly, request_logs,
    response_logs,
};

/// 데이터베이스 초기화
pub async fn initialize_db(config: &DbConfig, pool: &DatabasePool) -> Result<()> {
    debug!("데이터베이스 파티션 확인");
    match ensure_partitions(config, pool).await {
        Ok(_) => debug!("데이터베이스 파티션 확인 완료"),
        Err(e) => warn!("데이터베이스 파티션 확인 실패: {:?}", e),
    }
    Ok(())
}

/// 파티션 상태 확인
async fn ensure_partitions(config: &DbConfig, pool: &DatabasePool) -> Result<()> {
    // 커넥션 풀에서 로드
    let conn = pool.get_connection().await?;

    // 테이블생성 확인
    if let Err(e) = create_tables(&conn).await {
        error!("테이블 생성중 오류발생: {}", e);
    } else {
        info!("테이블 생성 완료");
    }

    // 파티션생성 확인
    if let Err(e) = set_all_partitions(&conn, config).await {
        error!("파티션 생성중 오류발생: {}", e);
    } else {
        info!("파티션 생성 완료");
    }

    Ok(())
}

/// 테이블 생성, 인덱싱
async fn create_tables(conn: &deadpool_postgres::Object) -> Result<()> {
    // request_logs 테이블
    match conn.execute(request_logs::CREATE_TABLE, &[]).await {
        Ok(_) => {
            info!("request_logs 테이블 생성 완료");

            // 인덱싱
            // for index_query in request_logs::CREATE_INDICES {
            //     if let Err(e) = conn.execute(index_query, &[]).await {
            //         error!("request_logs 인덱스 생성 실패: {}", e);
            //     }
            // }
        }
        Err(e) => {
            error!("request_logs 테이블 생성중 오류 발생: {}", e);
        }
    }

    // response_logs
    match conn.execute(response_logs::CREATE_TABLE, &[]).await {
        Ok(_) => {
            info!("response_logs 테이블 생성 완료");

            // 인덱싱
            // for index_query in response_logs::CREATE_INDICES {
            //     if let Err(e) = conn.execute(index_query, &[]).await {
            //         error!("response_logs 인덱스 생성 실패: {}", e);
            //     }
            // }
        }
        Err(e) => {
            error!("response_logs 테이블 생성중 오류 발생: {}", e);
        }
    }

    // proxy_stats
    match conn.execute(proxy_stats::CREATE_TABLE, &[]).await {
        Ok(_) => {
            info!("proxy_stats 테이블 생성 완료");

            // 인덱싱
            // for index_query in proxy_stats::CREATE_INDICES {
            //     if let Err(e) = conn.execute(index_query, &[]).await {
            //         error!("proxy_stats 인덱스 생성 실패: {}", e);
            //     }
            // }
        }
        Err(e) => {
            error!("proxy_stats 테이블 생성중 오류 발생: {}", e);
        }
    }

    // proxy_stats_hourly
    match conn.execute(proxy_stats_hourly::CREATE_TABLE, &[]).await {
        Ok(_) => {
            info!("proxy_stats_hourly 테이블 생성 완료");

            // 인덱싱
            // for index_query in proxy_stats_hourly::CREATE_INDICES {
            //     if let Err(e) = conn.execute(index_query, &[]).await {
            //         error!("proxy_stats_hourly 인덱스 생성 실패: {}", e);
            //     }
            // }
        }
        Err(e) => {
            error!("proxy_stats_hourly 테이블 생성중 오류 발생: {}", e);
        }
    }

    // domain_blocks
    match conn.execute(domain_blocks::CREATE_TABLE, &[]).await {
        Ok(_) => {
            info!("domain_blocks 테이블 생성 완료");

            // 인덱싱
            for index_query in domain_blocks::CREATE_INDICES {
                if let Err(e) = conn.execute(index_query, &[]).await {
                    error!("domain_blocks 인덱스 생성 실패: {}", e);
                }
            }
        }
        Err(e) => {
            error!("domain_blocks 테이블 생성중 오류 발생: {}", e);
        }
    }

    // domain_pattern_blocks
    match conn.execute(domain_pattern_blocks::CREATE_TABLE, &[]).await {
        Ok(_) => {
            info!("domain_pattern_blocks 테이블 생성 완료");

            // 인덱싱
            for index_query in domain_pattern_blocks::CREATE_INDICES {
                if let Err(e) = conn.execute(index_query, &[]).await {
                    error!("domain_pattern_blocks 인덱스 생성 실패: {}", e);
                }
            }
        }
        Err(e) => {
            error!("domain_pattern_blocks 테이블 생성중 오류 발생: {}", e);
        }
    }

    Ok(())
}

/// 파티션 생성
async fn set_all_partitions(conn: &deadpool_postgres::Object, config: &DbConfig) -> Result<()> {
    let today = chrono::Local::now().date_naive();
    let future_partitions = config.partitioning.future_partitions as i32;

    // request_logs 파티셔닝
    debug!("request_logs 파티션 생성");
    match create_partitions(conn, TableType::RequestLogs, today, future_partitions + 1).await {
        Ok(_) => info!("request_logs 파티션 생성완료"),
        Err(e) => error!("request_logs 파티션 생성 실패: {}", e),
    }

    // response_logs 파티셔닝
    debug!("response_logs 파티션 생성");
    match create_partitions(conn, TableType::ResponseLogs, today, future_partitions + 1).await {
        Ok(_) => info!("response_logs 파티션 생성완료"),
        Err(e) => error!("response_logs 파티션 생성 실패: {}", e),
    }

    // proxy_stats 파티셔닝
    debug!("proxy_stats 파티션 생성");
    match create_partitions(conn, TableType::ProxyStats, today, future_partitions + 1).await {
        Ok(_) => info!("proxy_stats 파티션 생성완료"),
        Err(e) => error!("proxy_stats 파티션 생성 실패: {}", e),
    }

    // proxy_stats_hourly 파티셔닝
    debug!("proxy_stats_hourly 파티션 생성");
    match create_partitions(
        conn,
        TableType::ProxyStatsHourly,
        today,
        future_partitions + 1,
    )
    .await
    {
        Ok(_) => info!("proxy_stats_hourly 파티션 생성완료"),
        Err(e) => error!("proxy_stats_hourly 파티션 생성 실패: {}", e),
    }

    Ok(())
}

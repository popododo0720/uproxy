/// 테이블 생성 쿼리
pub const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS proxy_stats_hourly (
        id SERIAL,
        timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        http_connections_avg DOUBLE PRECISION NOT NULL,
        http_bytes_in DOUBLE PRECISION NOT NULL,
        http_bytes_out DOUBLE PRECISION NOT NULL,
        tls_connections_avg DOUBLE PRECISION NOT NULL,
        tls_bytes_in DOUBLE PRECISION NOT NULL,
        tls_bytes_out DOUBLE PRECISION NOT NULL,
        uptime_seconds BIGINT NOT NULL,
        PRIMARY KEY (id, timestamp)
    ) PARTITION BY RANGE (timestamp)";

/// 기본 인덱스 생성 쿼리 - 부모 테이블에만 적용
pub const CREATE_INDICES: [&str; 1] = [
    "CREATE INDEX IF NOT EXISTS proxy_stats_hourly_timestamp_idx ON proxy_stats_hourly(timestamp)"
];
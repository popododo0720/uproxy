/// 테이블 생성 쿼리
pub const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS proxy_stats (
        id SERIAL,
        timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        http_active_connections BIGINT NOT NULL,
        http_bytes_in DOUBLE PRECISION NOT NULL,
        http_bytes_out DOUBLE PRECISION NOT NULL,
        tls_active_connections BIGINT NOT NULL,
        tls_bytes_in DOUBLE PRECISION NOT NULL,
        tls_bytes_out DOUBLE PRECISION NOT NULL,
        uptime_seconds BIGINT NOT NULL,
        seconds_since_reset BIGINT NOT NULL DEFAULT 0,
        PRIMARY KEY (id, timestamp)
    ) PARTITION BY RANGE (timestamp)";

/// 기본 인덱스 생성 쿼리
pub const CREATE_INDICES: [&str; 1] =
    ["CREATE INDEX IF NOT EXISTS proxy_stats_timestamp_idx ON proxy_stats(timestamp)"];

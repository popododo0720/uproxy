/// 테이블 생성 쿼리
pub const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS request_logs (
        id BIGSERIAL,
        host TEXT NOT NULL,
        method TEXT NOT NULL,
        path TEXT NOT NULL,
        header TEXT NOT NULL,
        body TEXT,
        timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        session_id TEXT NOT NULL,
        client_ip TEXT NOT NULL,
        target_ip TEXT NOT NULL,
        is_rejected BOOLEAN NOT NULL DEFAULT FALSE,
        is_tls BOOLEAN NOT NULL DEFAULT FALSE,
        PRIMARY KEY (id, timestamp)
    ) PARTITION BY RANGE (timestamp)";

/// 기본 인덱스 생성 쿼리
pub const CREATE_INDICES: [&str; 6] = [
    "CREATE INDEX IF NOT EXISTS request_logs_host_idx ON request_logs(host)",
    "CREATE INDEX IF NOT EXISTS request_logs_timestamp_idx ON request_logs(timestamp)",
    "CREATE INDEX IF NOT EXISTS request_logs_is_rejected_idx ON request_logs(is_rejected)",
    "CREATE INDEX IF NOT EXISTS request_logs_is_tls_idx ON request_logs(is_tls)",
    "CREATE INDEX IF NOT EXISTS request_logs_client_ip_idx ON request_logs(client_ip)",
    "CREATE INDEX IF NOT EXISTS request_logs_target_ip_idx ON request_logs(target_ip)",
];

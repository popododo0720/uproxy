/// 테이블 생성 쿼리
pub const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS response_logs (
        id BIGSERIAL,
        session_id TEXT NOT NULL,
        status_code INTEGER NOT NULL,
        response_time BIGINT NOT NULL,
        response_size BIGINT NOT NULL,
        timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        headers TEXT NOT NULL,
        body_preview TEXT,
        PRIMARY KEY (id, timestamp)
    ) PARTITION BY RANGE (timestamp)";

/// 기본 인덱스 생성 쿼리 
pub const CREATE_INDICES: [&str; 3] = [
    "CREATE INDEX IF NOT EXISTS response_logs_session_id_idx ON response_logs(session_id)",
    "CREATE INDEX IF NOT EXISTS response_logs_timestamp_idx ON response_logs(timestamp)",
    "CREATE INDEX IF NOT EXISTS response_logs_status_code_idx ON response_logs(status_code)"
];
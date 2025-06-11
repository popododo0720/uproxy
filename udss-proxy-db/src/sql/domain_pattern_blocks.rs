/// 테이블 생성 쿼리
pub const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS domain_pattern_blocks (
        id BIGSERIAL PRIMARY KEY,
        pattern VARCHAR(255) NOT NULL,
        created_by VARCHAR(100) NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        description TEXT,
        active BOOLEAN NOT NULL DEFAULT TRUE
    )
";

/// 인덱스 생성 쿼리
pub const CREATE_INDICES: [&str; 2] = [
    "CREATE INDEX IF NOT EXISTS domain_pattern_blocks_pattern_idx ON domain_pattern_blocks(pattern)",
    "CREATE INDEX IF NOT EXISTS domain_pattern_blocks_active_idx ON domain_pattern_blocks(active)",
];

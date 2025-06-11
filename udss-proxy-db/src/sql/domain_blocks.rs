/// 테이블 생성 쿼리
pub const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS domain_blocks (
        id BIGSERIAL PRIMARY KEY,
        domain VARCHAR(255) NOT NULL,
        created_by VARCHAR(100) NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        description TEXT,
        active BOOLEAN NOT NULL DEFAULT TRUE
    )
";

/// 인덱스 생성 쿼리
pub const CREATE_INDICES: [&str; 2] = [
    "CREATE INDEX IF NOT EXISTS domain_blocks_domain_idx ON domain_blocks(domain)",
    "CREATE INDEX IF NOT EXISTS domain_blocks_active_idx ON domain_blocks(active)",
];

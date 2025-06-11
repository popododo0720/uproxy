/// 도메인 목록 조회 쿼리
pub const SELECT_ACTIVE_DOMAINS: &str = "
    SELECT domain
    FROM domain_blocks
    WHERE active = TRUE
    ORDER BY domain
";

/// 패턴 목록 조회 쿼리
pub const SELECT_ACTIVE_PATTERNS: &str = "
    SELECT pattern
    FROM domain_pattern_blocks
    WHERE active = TRUE
    ORDER BY pattern
";

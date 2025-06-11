use std::collections::HashSet;
use std::sync::RwLock;

use log::{debug, error, info};
use regex::Regex;

use udss_proxy_db::pool::DatabasePool;
use udss_proxy_error::{ProxyError, Result};

use crate::sql;

/// 도메인 차단을 처리하는 구조체
pub struct DomainBlocker {
    // 차단된 도메인 목록
    blocked_domains: RwLock<HashSet<String>>,
    // 정규표현식 패턴
    regex_patterns: RwLock<Vec<Regex>>,
}

impl Default for DomainBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainBlocker {
    /// 새로운 `DomainBlocker` 인스턴스 생성
    pub fn new() -> Self {
        Self {
            blocked_domains: RwLock::new(HashSet::new()),
            regex_patterns: RwLock::new(Vec::new()),
        }
    }

    /// 초기화
    pub async fn init(&self, pool: &DatabasePool) -> Result<()> {
        // 커넥션 풀에서 로드
        let conn = pool.get_connection().await?;

        self.clear_blocklists()?;
        self.load_domains_from_db(&conn).await?;
        self.load_domain_patterns_from_db(&conn).await?;

        Ok(())
    }

    /// 도메인 차단여부
    pub fn is_blocked(&self, host: &str) -> bool {
        if host.is_empty() {
            return false;
        }

        match self.blocked_domains.read() {
            Ok(guard) => {
                if guard.contains(host) {
                    debug!("정확히 차단된 도메인: {host}");
                    return true;
                }
            }
            Err(e) => {
                error!("blocked_domains RwLock 읽기 잠금 실패 (is_blocked): {e}");
                return false;
            }
        }

        match self.regex_patterns.read() {
            Ok(guard) => {
                for pattern in guard.iter() {
                    if pattern.is_match(host) {
                        debug!("패턴으로 차단된 도메인: {} ({})", host, pattern.as_str());
                        return true;
                    }
                }
            }
            Err(e) => {
                error!("regex_patterns RwLock 읽기 잠금 실패 (is_blocked): {e}");
                return false;
            }
        }

        false
    }

    /// 기존목록 초기화
    fn clear_blocklists(&self) -> Result<()> {
        match self.blocked_domains.write() {
            Ok(mut guard) => guard.clear(),
            Err(e) => {
                let err_msg = format!("blocked_domains RwLock 쓰기 잠금 실패 (초기화 중): {e}");
                error!("{err_msg}");
                return Err(ProxyError::Internal(err_msg));
            }
        }
        match self.regex_patterns.write() {
            Ok(mut guard) => guard.clear(),
            Err(e) => {
                let err_msg = format!("regex_patterns RwLock 쓰기 잠금 실패 (초기화 중): {e}");
                error!("{err_msg}");
                return Err(ProxyError::Internal(err_msg));
            }
        }
        info!("기존 도메인 차단 목록 초기화 완료.");
        Ok(())
    }

    /// 도메인 차단목록
    async fn load_domains_from_db(&self, conn: &deadpool_postgres::Object) -> Result<()> {
        debug!("데이터베이스에서 도메인 차단 목록 로드 중...");

        let pg_rows = conn
            .query(sql::SELECT_ACTIVE_DOMAINS, &[])
            .await
            .map_err(|e| {
                error!("도메인 차단 목록 쿼리 실패: {e}");
                ProxyError::Database(format!("DB query error: {e}"))
            })?;

        let mut blocked_domains_writer = self.blocked_domains.write().map_err(|e| {
            let err_msg = format!("blocked_domains RwLock 쓰기 잠금 실패 (DB 로드 중): {e}");
            error!("{err_msg}");
            ProxyError::Internal(err_msg)
        })?;

        for row in pg_rows {
            match row.try_get::<usize, String>(0) {
                Ok(domain) => {
                    debug!("차단 목록에 도메인 추가: {domain}");
                    blocked_domains_writer.insert(domain);
                }
                Err(e) => {
                    error!("DB 행에서 도메인 문자열 추출 실패: {e}");
                }
            }
        }

        info!(
            "도메인 차단 목록 로드 완료. {}개의 도메인 로드",
            blocked_domains_writer.len()
        );
        Ok(())
    }

    /// 도메인 패턴 차단목록
    async fn load_domain_patterns_from_db(&self, conn: &deadpool_postgres::Object) -> Result<()> {
        debug!("데이터베이스에서 도메인 차단 패턴 목록 로드 중...");

        let pg_rows = conn
            .query(sql::SELECT_ACTIVE_PATTERNS, &[])
            .await
            .map_err(|e| {
                error!("도메인 차단 패턴 목록 쿼리 실패: {e}");
                ProxyError::Database(format!("DB query error: {e}"))
            })?;

        let mut regex_patterns_writer = self.regex_patterns.write().map_err(|e| {
            let err_msg = format!("regex_patterns RwLock 쓰기 잠금 실패 (DB 로드 중): {e}");
            error!("{err_msg}");
            ProxyError::Internal(err_msg)
        })?;

        for row in pg_rows {
            match row.try_get::<usize, String>(0) {
                Ok(pattern_str) => match Regex::new(&pattern_str) {
                    Ok(regex) => {
                        debug!("차단 패턴 목록에 정규식 추가: {pattern_str}");
                        regex_patterns_writer.push(regex);
                    }
                    Err(e) => {
                        error!("정규식 컴파일 실패 '{pattern_str}': {e}");
                    }
                },
                Err(e) => {
                    error!("DB 행에서 패턴 문자열 추출 실패: {e}");
                }
            }
        }

        info!(
            "도메인 차단 패턴 목록 로드 완료. {}개의 패턴 로드됨.",
            regex_patterns_writer.len()
        );

        Ok(())
    }
}

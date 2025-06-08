use std::path::Path;

use log::{info};

use udss_proxy_error::{ProxyError, Result};

use crate::config::Config;
use crate::dbconfig::DbConfig;

/// 통합 세팅 인스턴스
pub struct Settings {
    pub proxy: Config,
    pub database: DbConfig,
}

impl Settings {
    /// Setting 생성
    pub fn new() -> Result<Self> {
        let proxy = Self::load_proxy_config()?;
        let database = Self::load_db_config()?;

        Ok(Self {
            proxy,
            database,
        })
    }

    /// 프록시 설정 로드
    fn load_proxy_config() -> Result<Config> {
        // yml 파일 유무 확인
        if Path::new("config.yml").exists() {
            info!("프록시 설정파일 로드: config.yml");
            match Config::from_file("config.yml") {
                Ok(config) => Ok(config),
                Err(e) => {
                    Err(ProxyError::Config(format!("프록시 설정파일 로드 실패: {}", e)))
                }
            }
        } else {
            // 기본설정사용
            info!("프록시 기본설정 사용");
            Ok(Config::new())
        }
    }

    /// db 설정 로드
    fn load_db_config() -> Result<DbConfig> {
        // yml 파일 유무 확인
        if Path::new("db.yml").exists() {
            info!("DB 설정파일 로드: db.yml");
            match DbConfig::from_file("db.yml") {
                Ok(config) => Ok(config),
                Err(e) => {
                    Err(ProxyError::Config(format!("DB 설정파일 로드 실패: {}", e)))
                }
            }
        } else {
            // 기본설정사용
            info!("DB 기본설정 사용");
            Ok(DbConfig::default())
        }
    }
}
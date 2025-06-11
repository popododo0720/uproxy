use std::fs;
use std::path::Path;

use log::{debug, info, warn};
use once_cell::sync::Lazy;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
};
use tokio::sync::Mutex;
use udss_proxy_config::Config;
use udss_proxy_error::{Result, config_err};

// 루트 CA 인증서 및 키 저장 (전역 변수)
pub static ROOT_CA: Lazy<Mutex<Option<Certificate>>> = Lazy::new(|| Mutex::new(None));

// 인증서 파일 경로 상수
const CA_CERT_PEM_FILE: &str = "ca_cert.pem";
const CA_KEY_PEM_FILE: &str = "ca_key.pem";
const CA_CERT_CRT_FILE: &str = "ca_cert.crt";

/// 인증서 디렉토리 확인 및 생성
pub fn ensure_ssl_directories(config: &Config) -> Result<()> {
    debug!("인증서 디렉토리 확인 및 생성");

    let ssl_dir = &config.ssl_dir;
    // let cert_dir = format!("{}/certs", ssl_dir);
    // let key_dir = format!("{}/private", ssl_dir);
    let trusted_dir = format!("{}/trusted_certs", ssl_dir);

    for dir in &[ssl_dir, &trusted_dir] {
        if !Path::new(dir).exists() {
            std::fs::create_dir_all(dir)?;
            info!("디렉토리 생성: {}", dir);
        }
    }

    // for dir in &[ssl_dir, &cert_dir, &key_dir, &trusted_dir] {
    //     if !Path::new(dir).exists() {
    //         std::fs::create_dir_all(dir).map_err(|e| ProxyError::from(e))?;
    //         info!("디렉토리 생성: {}", dir);
    //     }
    // }

    Ok(())
}

/// 신뢰할수있는 인증서 로드
pub fn load_trusted_certificates(config: &mut Config) -> Result<()> {
    debug!("신뢰할수있는 인증서 로드");

    // 인증서 디렉토리 확인
    let cert_dir = Path::new(&config.ssl_dir).join("trusted_certs");
    if !cert_dir.exists() || !cert_dir.is_dir() {
        warn!("인증서 디렉토리가 존재하지 않습니다: {}", config.ssl_dir);
        return Ok(());
    }

    let cert_files = std::fs::read_dir(cert_dir)
        .map_err(|e| config_err(format!("인증서 디렉토리 읽기 실패: {}", e)))?;

    for entry in cert_files.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == "pem" || ext == "crt")
        {
            if let Some(path_str) = path.to_str() {
                println!("Valid certificate file: {}", path_str);
            }
        }
    }

    info!("총 {} 개의 인증서 로드", config.trusted_certificates.len());
    Ok(())
}

/// 루트 CA 인증서 초기화
pub async fn init_root_ca(config: &Config) -> Result<()> {
    let mut ca_guard = ROOT_CA.lock().await;

    let ssl_dir = &config.ssl_dir;
    let ca_cert_pem_path = format!("{}/{}", ssl_dir, CA_CERT_PEM_FILE);
    let ca_key_pem_path = format!("{}/{}", ssl_dir, CA_KEY_PEM_FILE);
    let ca_cert_crt_path = format!("{}/{}", ssl_dir, CA_CERT_CRT_FILE);

    // 기존 인증서 유무 확인
    let crt_exists = Path::new(&ca_cert_crt_path).exists();
    let key_exists = Path::new(&ca_key_pem_path).exists();
    let pem_exists = Path::new(&ca_cert_pem_path).exists();

    if crt_exists && key_exists && pem_exists {
        info!("기존 CA 인증서 로드");
        let _cert_pem = fs::read_to_string(&ca_cert_pem_path)?;
        let key_pem = fs::read_to_string(&ca_key_pem_path)?;

        // PEM에서 인증서와 키 로드
        let key_pair = KeyPair::from_pem(&key_pem)?;

        // 인증서 파라미터 생성
        let params = CertificateParams::new(vec!["UDSS Proxy Root CA".to_string()])?;

        // 인증서 생성
        let cert = params.self_signed(&key_pair)?;

        *ca_guard = Some(cert);
    } else if crt_exists && key_exists {
        // .crt와 .key 파일이 모두 존재하는 경우 .pem 파일 생성
        info!(".crt와 .key 파일에서 .pem 파일 생성");
        let cert_crt = fs::read_to_string(&ca_cert_crt_path)?;
        let key_pem = fs::read_to_string(&ca_key_pem_path)?;

        // .crt 내용과 .key 내용을 합쳐서 .pem 파일 생성
        fs::write(&ca_cert_pem_path, format!("{}{}", cert_crt, key_pem))?;
        info!("CA 인증서 PEM 생성 완료: {}", ca_cert_pem_path);

        // 키페어와 인증서 로드
        let key_pair = KeyPair::from_pem(&key_pem)?;

        // 인증서 파라미터 생성
        let params = CertificateParams::new(vec!["UDSS Proxy Root CA".to_string()])?;

        // 인증서 생성
        let cert = params.self_signed(&key_pair)?;

        *ca_guard = Some(cert);
    } else if pem_exists {
        // .pem 파일만 존재하는 경우 .crt와 .key 파일로 분리
        info!(".pem 파일에서 .crt와 .key 파일 생성");
        let pem_content = fs::read_to_string(&ca_cert_pem_path)?;

        // PEM 파일에서 인증서와 키 부분 분리
        // 일반적으로 PEM 파일은 인증서 부분과 키 부분으로 구성됨
        // 인증서 부분은 "-----BEGIN CERTIFICATE-----"로 시작
        // 키 부분은 "-----BEGIN PRIVATE KEY-----"로 시작

        let cert_part: String;
        let key_part: String;

        if let Some(key_start) = pem_content.find("-----BEGIN PRIVATE KEY-----") {
            cert_part = pem_content[0..key_start].trim().to_string();
            key_part = pem_content[key_start..].trim().to_string();
        } else if let Some(key_start) = pem_content.find("-----BEGIN RSA PRIVATE KEY-----") {
            cert_part = pem_content[0..key_start].trim().to_string();
            key_part = pem_content[key_start..].trim().to_string();
        } else {
            return Err(config_err(
                "PEM 파일에서 인증서와 키를 분리할 수 없습니다.".to_string(),
            ));
        }

        // 파일로 저장
        fs::write(&ca_cert_crt_path, &cert_part)?;
        fs::write(&ca_key_pem_path, &key_part)?;

        info!("CA 인증서 CRT 생성 완료: {}", ca_cert_crt_path);
        info!("CA 키 생성 완료: {}", ca_key_pem_path);

        // 키페어와 인증서 로드
        let key_pair = KeyPair::from_pem(&key_part)?;

        // 인증서 파라미터 생성
        let params = CertificateParams::new(vec!["UDSS Proxy Root CA".to_string()])?;

        // 인증서 생성
        let cert = params.self_signed(&key_pair)?;

        *ca_guard = Some(cert);
    } else {
        info!("새 CA 인증서 생성");

        // 인증서 파라미터 설정
        let mut params = CertificateParams::default();
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, "UDSS Proxy Root CA");
        distinguished_name.push(DnType::OrganizationName, "CoremaxTech");
        distinguished_name.push(DnType::CountryName, "KR");
        params.distinguished_name = distinguished_name;

        // 인증서 속성 설정 - IsCa enum 사용
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
            rcgen::KeyUsagePurpose::DigitalSignature,
        ];

        // 키페어 생성 및 인증서 생성
        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;

        // PEM 형식으로 저장
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        // 파일로 저장
        fs::write(&ca_cert_pem_path, &cert_pem)?;
        fs::write(&ca_key_pem_path, &key_pem)?;

        // Windows 인증서 스토어용 .crt 파일 생성
        fs::write(&ca_cert_crt_path, &cert_pem)?;

        *ca_guard = Some(cert);
    }

    Ok(())
}

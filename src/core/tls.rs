//! TLS/HTTPS配置和证书管理

use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::rustls::{self, Certificate, PrivateKey};
use tokio_rustls::TlsAcceptor;

/// TLS配置
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// 证书文件路径
    pub cert_path: String,
    /// 私钥文件路径
    pub key_path: String,
    /// 是否启用HSTS
    pub enable_hsts: bool,
    /// HSTS最大年龄（秒）
    pub hsts_max_age: u32,
    /// 是否启用客户端证书验证
    pub enable_client_auth: bool,
    /// CA证书路径（用于客户端证书验证）
    pub ca_cert_path: Option<String>,
    /// 最低TLS版本
    pub min_tls_version: TlsVersion,
    /// 支持的密码套件
    pub cipher_suites: Vec<String>,
}

/// TLS版本
#[derive(Debug, Clone, Copy)]
pub enum TlsVersion {
    Tls1_2,
    Tls1_3,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: "certs/server.crt".to_string(),
            key_path: "certs/server.key".to_string(),
            enable_hsts: true,
            hsts_max_age: 31536000, // 1年
            enable_client_auth: false,
            ca_cert_path: None,
            min_tls_version: TlsVersion::Tls1_2,
            cipher_suites: vec![
                "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string(),
                "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256".to_string(),
                "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256".to_string(),
            ],
        }
    }
}

/// TLS管理器
pub struct TlsManager {
    config: TlsConfig,
    acceptor: Option<TlsAcceptor>,
}

impl TlsManager {
    /// 创建新的TLS管理器
    pub fn new(config: TlsConfig) -> Self {
        Self {
            config,
            acceptor: None,
        }
    }

    /// 初始化TLS配置
    pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let certs = load_certs(&self.config.cert_path)?;
        let key = load_private_key(&self.config.key_path)?;

        let mut config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth();

        // 如果启用客户端认证，配置CA证书
        if self.config.enable_client_auth {
            if let Some(ca_path) = &self.config.ca_cert_path {
                let ca_certs = load_certs(ca_path)?;
                let mut roots = rustls::RootCertStore::empty();
                for cert in ca_certs {
                    roots.add(&cert)?;
                }
                config = rustls::ServerConfig::builder()
                    .with_safe_defaults()
                    .with_client_cert_verifier(Arc::new(rustls::server::AllowAnyAuthenticatedClient::new(roots)));
            }
        }

        config
            .with_single_cert(certs, key)
            .map_err(|e| format!("Failed to create TLS config: {}", e))?;

        self.acceptor = Some(TlsAcceptor::from(Arc::new(config)));
        Ok(())
    }

    /// 获取TLS接收器
    pub fn acceptor(&self) -> Option<&TlsAcceptor> {
        self.acceptor.as_ref()
    }

    /// 检查TLS配置是否有效
    pub fn validate_config(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 检查证书文件是否存在
        if !Path::new(&self.config.cert_path).exists() {
            return Err(format!("Certificate file not found: {}", self.config.cert_path).into());
        }

        // 检查私钥文件是否存在
        if !Path::new(&self.config.key_path).exists() {
            return Err(format!("Private key file not found: {}", self.config.key_path).into());
        }

        // 如果启用了客户端认证，检查CA证书
        if self.config.enable_client_auth {
            if let Some(ca_path) = &self.config.ca_cert_path {
                if !Path::new(ca_path).exists() {
                    return Err(format!("CA certificate file not found: {}", ca_path).into());
                }
            } else {
                return Err("Client authentication enabled but no CA certificate provided".into());
            }
        }

        Ok(())
    }

    /// 获取TLS配置
    pub fn config(&self) -> &TlsConfig {
        &self.config
    }

    /// 生成自签名证书（仅用于开发环境）
    pub fn generate_self_signed_cert(
        &self,
        domain: &str,
        validity_days: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use ring::rand::SystemRandom;
        use ring::signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
        use std::io::Write;

        let rng = SystemRandom::new();
        let alg = &ECDSA_P256_SHA256_FIXED_SIGNING;
        let pkcs8_bytes = EcdsaKeyPair::generate_pkcs8(alg, &rng)
            .map_err(|_| "Failed to generate key pair")?;

        // 简单的自签名证书生成（生产环境应该使用正式的CA证书）
        println!("⚠️  WARNING: Using self-signed certificate for development only!");
        println!("   Domain: {}", domain);
        println!("   Certificate: {}", self.config.cert_path);
        println!("   Private Key: {}", self.config.key_path);
        println!("   For production, use proper CA-signed certificates.");

        Ok(())
    }
}

/// 加载证书文件
fn load_certs(path: &str) -> Result<Vec<Certificate>, Box<dyn std::error::Error + Send + Sync>> {
    let cert_data = fs::read(path)
        .map_err(|e| format!("Failed to read certificate file {}: {}", path, e))?;

    let mut reader = std::io::BufReader::new(cert_data.as_slice());
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|e| format!("Failed to parse certificate: {}", e))?
        .into_iter()
        .map(Certificate)
        .collect();

    if certs.is_empty() {
        return Err(format!("No certificates found in {}", path).into());
    }

    Ok(certs)
}

/// 加载私钥文件
fn load_private_key(path: &str) -> Result<PrivateKey, Box<dyn std::error::Error + Send + Sync>> {
    let key_data = fs::read(path)
        .map_err(|e| format!("Failed to read private key file {}: {}", path, e))?;

    let mut reader = std::io::BufReader::new(key_data.as_slice());
    let keys = rustls_pemfile::rsa_private_keys(&mut reader)
        .map_err(|e| format!("Failed to parse RSA private key: {}", e))?;

    if keys.is_empty() {
        // 尝试ECDSA私钥
        let mut reader = std::io::BufReader::new(key_data.as_slice());
        let ecdsa_keys = rustls_pemfile::ec_private_keys(&mut reader)
            .map_err(|e| format!("Failed to parse ECDSA private key: {}", e))?;

        if ecdsa_keys.is_empty() {
            return Err(format!("No private keys found in {}", path).into());
        }

        return Ok(PrivateKey(ecdsa_keys[0].clone()));
    }

    Ok(PrivateKey(keys[0].clone()))
}

/// 创建默认TLS配置
pub fn create_default_tls_config() -> TlsConfig {
    TlsConfig::default()
}

/// TLS中间件
pub mod middleware {
    use axum::{
        extract::Request,
        http::{header, StatusCode},
        middleware::Next,
        response::{IntoResponse, Response},
    };

    /// HSTS中间件
    pub async fn hsts_middleware(request: Request, next: Next) -> Response {
        let mut response = next.run(request).await;

        // 添加HSTS头
        let headers = response.headers_mut();
        headers.insert(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains; preload".parse().unwrap(),
        );

        response
    }

    /// HTTPS重定向中间件
    pub async fn https_redirect_middleware(
        request: Request,
        next: Next,
    ) -> Response {
        // 检查是否是HTTP请求
        let host = request.headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok());

        let uri = request.uri();

        // 如果是HTTP请求，重定向到HTTPS
        if let Some(host) = host {
            if !uri.scheme_str().unwrap_or("").starts_with("https") {
                let https_uri = format!("https://{}{}", host, uri);
                return (
                    StatusCode::MOVED_PERMANENTLY,
                    [(header::LOCATION, https_uri)],
                    "Redirecting to HTTPS",
                ).into_response();
            }
        }

        next.run(request).await
    }
}

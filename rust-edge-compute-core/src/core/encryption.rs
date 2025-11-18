//! 数据加密和安全存储

use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use aes_gcm::aead::{Aead};
use argon2::{Argon2, PasswordHasher, password_hash::{PasswordHash, PasswordVerifier, Salt, SaltString}};
use rand::RngCore;
use ring::rand::SecureRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 加密配置
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// 密钥派生算法
    pub kdf_algorithm: String,
    /// 加密算法
    pub cipher_algorithm: String,
    /// 密钥长度
    pub key_length: usize,
    /// 盐长度
    pub salt_length: usize,
    /// 工作因子
    pub work_factor: u32,
    /// 并行度
    pub parallelism: u32,
    /// 内存大小（KB）
    pub memory_size_kb: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            kdf_algorithm: "argon2id".to_string(),
            cipher_algorithm: "aes-256-gcm".to_string(),
            key_length: 32, // 256位
            salt_length: 32,
            work_factor: 3,
            parallelism: 4,
            memory_size_kb: 65536, // 64MB
        }
    }
}

/// 加密管理器
pub struct EncryptionManager {
    config: EncryptionConfig,
    master_key: Option<Vec<u8>>,
    key_cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl EncryptionManager {
    /// 创建新的加密管理器
    pub fn new(config: EncryptionConfig) -> Self {
        Self {
            config,
            master_key: None,
            key_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 设置主密钥
    pub fn set_master_key(&mut self, key: Vec<u8>) {
        self.master_key = Some(key);
    }

    /// 从密码派生主密钥
    pub fn derive_master_key(&mut self, password: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let salt = SaltString::generate(&mut rand::thread_rng());

        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(
            password.as_bytes(),
            &salt,
        ).map_err(|e| format!("Failed to hash password: {}", e))?;

        let hash = password_hash.hash.unwrap();
        let key = hash.as_bytes().to_vec();

        if key.len() != self.config.key_length {
            return Err(format!("Derived key length {} does not match expected length {}",
                             key.len(), self.config.key_length).into());
        }

        self.master_key = Some(key);
        Ok(())
    }

    /// 加密数据
    pub fn encrypt(&self, data: &[u8], key_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.get_or_create_key(key_id)?;
        let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from_slice(&key));

        // 生成随机nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密数据
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // 组合nonce和密文
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// 解密数据
    pub fn decrypt(&self, encrypted_data: &[u8], key_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if encrypted_data.len() < 12 {
            return Err("Encrypted data too short".into());
        }

        let key = self.get_or_create_key(key_id)?;
        let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from_slice(&key));

        // 提取nonce和密文
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        // 解密数据
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }

    /// 加密字符串
    pub fn encrypt_string(&self, text: &str, key_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let data = text.as_bytes();
        let encrypted = self.encrypt(data, key_id)?;
        Ok(base64::encode(&encrypted))
    }

    /// 解密字符串
    pub fn decrypt_string(&self, encrypted_text: &str, key_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let encrypted_data = base64::decode(encrypted_text)
            .map_err(|e| format!("Failed to decode base64: {}", e))?;
        let decrypted = self.decrypt(&encrypted_data, key_id)?;
        String::from_utf8(decrypted)
            .map_err(|e| format!("Failed to decode UTF-8: {}", e).into())
    }

    /// 获取或创建密钥
    fn get_or_create_key(&self, key_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut cache = self.key_cache.try_lock()
            .map_err(|_| "Failed to acquire key cache lock")?;

        if let Some(key) = cache.get(key_id) {
            return Ok(key.clone());
        }

        // 如果缓存中没有，创建一个新的密钥
        let master_key = self.master_key.as_ref()
            .ok_or("Master key not set")?;

        // 使用master_key和key_id派生数据密钥
        let mut key_material = master_key.clone();
        key_material.extend_from_slice(key_id.as_bytes());

        // 使用HKDF派生密钥
        let salt = b"encryption_key_salt";
        let mut derived_key = vec![0u8; self.config.key_length];

        ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, salt)
            .extract(&key_material)
            .expand(&[b"encryption"], ring::hkdf::HKDF_SHA256)
            .map_err(|_| "HKDF expansion failed")?
            .fill(&mut derived_key)
            .map_err(|_| "Failed to derive key")?;

        cache.insert(key_id.to_string(), derived_key.clone());
        Ok(derived_key)
    }

    /// 验证密码
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| format!("Failed to parse password hash: {}", e))?;

        let argon2 = Argon2::default();
        let result = argon2.verify_password(password.as_bytes(), &parsed_hash);

        Ok(result.is_ok())
    }

    /// 哈希密码
    pub fn hash_password(&self, password: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();

        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {}", e))?;

        Ok(password_hash.to_string())
    }

    /// 生成随机密钥
    pub fn generate_random_key(length: usize) -> Vec<u8> {
        let mut key = vec![0u8; length];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// 生成随机token
    pub fn generate_token(length: usize) -> String {
        let bytes = Self::generate_random_key(length);
        base64::encode(&bytes)
    }

    /// 清理密钥缓存
    pub async fn clear_key_cache(&self) {
        let mut cache = self.key_cache.lock().await;
        cache.clear();
    }

    /// 获取缓存中的密钥数量
    pub async fn key_cache_size(&self) -> usize {
        let cache = self.key_cache.lock().await;
        cache.len()
    }
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new(EncryptionConfig::default())
    }
}

/// 安全存储接口
#[async_trait::async_trait]
pub trait SecureStorage {
    /// 安全存储数据
    async fn secure_store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 安全读取数据
    async fn secure_load(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;

    /// 删除安全存储的数据
    async fn secure_delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// 基于文件的加密存储
pub struct EncryptedFileStorage {
    encryption_manager: Arc<EncryptionManager>,
    storage_path: String,
}

impl EncryptedFileStorage {
    pub fn new(encryption_manager: Arc<EncryptionManager>, storage_path: String) -> Self {
        Self {
            encryption_manager,
            storage_path,
        }
    }
}

#[async_trait::async_trait]
impl SecureStorage for EncryptedFileStorage {
    async fn secure_store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let encrypted_value = self.encryption_manager.encrypt_string(value, key)?;
        let file_path = format!("{}/{}", self.storage_path, key);

        tokio::fs::write(&file_path, encrypted_value).await
            .map_err(|e| format!("Failed to write encrypted file {}: {}", file_path, e).into())
    }

    async fn secure_load(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let file_path = format!("{}/{}", self.storage_path, key);

        match tokio::fs::read_to_string(&file_path).await {
            Ok(encrypted_value) => {
                let decrypted_value = self.encryption_manager.decrypt_string(&encrypted_value, key)?;
                Ok(Some(decrypted_value))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Failed to read encrypted file {}: {}", file_path, e).into()),
        }
    }

    async fn secure_delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file_path = format!("{}/{}", self.storage_path, key);

        match tokio::fs::remove_file(&file_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), // 文件不存在也算成功
            Err(e) => Err(format!("Failed to delete encrypted file {}: {}", file_path, e).into()),
        }
    }
}

/// 安全工具函数
pub mod utils {
    use super::*;

    /// 安全擦除内存中的敏感数据
    pub fn secure_zeroize(data: &mut [u8]) {
        ring::constant_time::verify_slices_are_equal(data, &[0u8; 32]).ok(); // 强制使用data
        data.fill(0);
    }

    /// 检查密码强度
    pub fn check_password_strength(password: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if password.len() < 8 {
            return Err("Password must be at least 8 characters long".into());
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if !has_uppercase || !has_lowercase || !has_digit || !has_special {
            return Err("Password must contain uppercase, lowercase, digit, and special characters".into());
        }

        Ok(())
    }

    /// 生成安全的随机数
    pub fn secure_random_bytes(length: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; length];
        ring::rand::SystemRandom::new().fill(&mut bytes).unwrap();
        bytes
    }

    /// 计算数据的哈希值
    pub fn hash_data(data: &[u8]) -> String {
        use ring::digest;
        let hash = digest::digest(&digest::SHA256, data);
        base64::encode(hash.as_ref())
    }
}

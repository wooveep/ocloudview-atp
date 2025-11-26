use thiserror::Error;

/// Storage 层错误类型
#[derive(Error, Debug)]
pub enum StorageError {
    /// 数据库连接错误
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    /// 数据库操作错误
    #[error("Database operation error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// 数据序列化错误
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// 数据未找到
    #[error("Data not found: {0}")]
    NotFound(String),

    /// 数据已存在
    #[error("Data already exists: {0}")]
    AlreadyExists(String),

    /// 数据验证错误
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// 迁移错误
    #[error("Migration error: {0}")]
    MigrationError(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

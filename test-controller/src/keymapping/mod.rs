pub mod mapper;
pub mod layout;

pub use mapper::KeyMapper;
pub use layout::{KeyboardLayout, KeyMapping};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyMappingError {
    #[error("不支持的字符: {0}")]
    UnsupportedCharacter(char),

    #[error("不支持的键盘布局: {0}")]
    UnsupportedLayout(String),

    #[error("映射失败: {0}")]
    MappingFailed(String),
}

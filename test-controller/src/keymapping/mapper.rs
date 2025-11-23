use std::collections::HashMap;

use super::layout::{build_us_layout_map, KeyMapping, KeyboardLayout};
use super::KeyMappingError;

/// 键值映射器
pub struct KeyMapper {
    layout: KeyboardLayout,
    mapping_table: HashMap<char, KeyMapping>,
}

impl KeyMapper {
    /// 创建指定布局的映射器
    pub fn new(layout: KeyboardLayout) -> Self {
        let mapping_table = match layout {
            KeyboardLayout::EnUS => build_us_layout_map(),
            // TODO: 实现其他布局
            _ => build_us_layout_map(),
        };

        Self {
            layout,
            mapping_table,
        }
    }

    /// 创建默认映射器（US 布局）
    pub fn default() -> Self {
        Self::new(KeyboardLayout::EnUS)
    }

    /// 将字符串映射为 QMP 按键序列
    pub fn map_string(&self, text: &str) -> Result<Vec<String>, KeyMappingError> {
        let mut qcodes = Vec::new();

        for ch in text.chars() {
            let mapping = self
                .mapping_table
                .get(&ch)
                .ok_or(KeyMappingError::UnsupportedCharacter(ch))?;

            // 将按键序列转换为 QCode 字符串
            for key_press in &mapping.keys {
                if key_press.press {
                    qcodes.push(key_press.qcode.clone());
                }
            }
        }

        Ok(qcodes)
    }

    /// 将单个字符映射为 QMP 按键序列
    pub fn map_char(&self, ch: char) -> Result<Vec<String>, KeyMappingError> {
        let mapping = self
            .mapping_table
            .get(&ch)
            .ok_or(KeyMappingError::UnsupportedCharacter(ch))?;

        let qcodes = mapping
            .keys
            .iter()
            .filter(|kp| kp.press)
            .map(|kp| kp.qcode.clone())
            .collect();

        Ok(qcodes)
    }

    /// 获取当前键盘布局
    pub fn layout(&self) -> KeyboardLayout {
        self.layout
    }

    /// 映射特殊键
    pub fn map_special_key(key_name: &str) -> Option<String> {
        let mapping = match key_name.to_lowercase().as_str() {
            "enter" | "return" => Some("ret"),
            "space" => Some("spc"),
            "tab" => Some("tab"),
            "backspace" => Some("backspace"),
            "delete" => Some("delete"),
            "escape" | "esc" => Some("esc"),
            "shift" => Some("shift"),
            "ctrl" | "control" => Some("ctrl"),
            "alt" => Some("alt"),
            "meta" | "super" | "win" => Some("meta_l"),
            "up" => Some("up"),
            "down" => Some("down"),
            "left" => Some("left"),
            "right" => Some("right"),
            "home" => Some("home"),
            "end" => Some("end"),
            "pageup" => Some("pgup"),
            "pagedown" => Some("pgdn"),
            "insert" => Some("insert"),
            _ => None,
        };

        mapping.map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_string() {
        let mapper = KeyMapper::default();

        let result = mapper.map_string("Hello");
        assert!(result.is_ok());

        let qcodes = result.unwrap();
        println!("Mapped 'Hello' to: {:?}", qcodes);
    }

    #[test]
    fn test_map_char() {
        let mapper = KeyMapper::default();

        let result = mapper.map_char('A');
        assert!(result.is_ok());

        let qcodes = result.unwrap();
        // 'A' 应该映射为 shift + a
        println!("Mapped 'A' to: {:?}", qcodes);
    }

    #[test]
    fn test_map_special_key() {
        assert_eq!(KeyMapper::map_special_key("enter"), Some("ret".to_string()));
        assert_eq!(KeyMapper::map_special_key("space"), Some("spc".to_string()));
        assert_eq!(KeyMapper::map_special_key("unknown"), None);
    }

    #[test]
    fn test_unsupported_character() {
        let mapper = KeyMapper::default();

        let result = mapper.map_char('中');
        assert!(result.is_err());
    }
}

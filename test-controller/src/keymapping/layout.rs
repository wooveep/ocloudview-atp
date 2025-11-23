use std::collections::HashMap;

/// 键盘布局枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardLayout {
    /// 美式键盘
    EnUS,
    /// 英式键盘
    EnGB,
    /// 中文键盘
    ZhCN,
}

impl KeyboardLayout {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "en-us" | "us" => Some(Self::EnUS),
            "en-gb" | "uk" => Some(Self::EnGB),
            "zh-cn" | "cn" => Some(Self::ZhCN),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EnUS => "en-US",
            Self::EnGB => "en-GB",
            Self::ZhCN => "zh-CN",
        }
    }
}

/// 按键映射：字符 -> QMP QCode 序列
#[derive(Debug, Clone)]
pub struct KeyMapping {
    /// 按键序列（可能包含修饰键）
    pub keys: Vec<KeyPress>,
}

/// 单次按键操作
#[derive(Debug, Clone)]
pub struct KeyPress {
    /// QMP QCode（如 "a", "shift-l", "ret"）
    pub qcode: String,
    /// 是否需要按下（true）或释放（false）
    pub press: bool,
}

impl KeyPress {
    pub fn press(qcode: &str) -> Self {
        Self {
            qcode: qcode.to_string(),
            press: true,
        }
    }

    pub fn release(qcode: &str) -> Self {
        Self {
            qcode: qcode.to_string(),
            press: false,
        }
    }

    /// 完整的按键操作（按下并释放）
    pub fn full(qcode: &str) -> Vec<Self> {
        vec![Self::press(qcode), Self::release(qcode)]
    }
}

/// 构建默认的 US 键盘映射表
pub fn build_us_layout_map() -> HashMap<char, KeyMapping> {
    let mut map = HashMap::new();

    // 小写字母 a-z
    for c in 'a'..='z' {
        map.insert(
            c,
            KeyMapping {
                keys: KeyPress::full(&c.to_string()),
            },
        );
    }

    // 大写字母 A-Z（需要 Shift）
    for c in 'A'..='Z' {
        let lower = c.to_lowercase().next().unwrap();
        map.insert(
            c,
            KeyMapping {
                keys: vec![
                    KeyPress::press("shift"),
                    KeyPress::press(&lower.to_string()),
                    KeyPress::release(&lower.to_string()),
                    KeyPress::release("shift"),
                ],
            },
        );
    }

    // 数字 0-9
    for c in '0'..='9' {
        map.insert(
            c,
            KeyMapping {
                keys: KeyPress::full(&c.to_string()),
            },
        );
    }

    // 特殊字符映射（US 布局）
    let special_chars = vec![
        (' ', "spc"),
        ('\n', "ret"),
        ('\t', "tab"),
        ('.', "dot"),
        (',', "comma"),
        ('/', "slash"),
        (';', "semicolon"),
        ('\'', "apostrophe"),
        ('[', "bracket_left"),
        (']', "bracket_right"),
        ('\\', "backslash"),
        ('-', "minus"),
        ('=', "equal"),
        ('`', "grave_accent"),
    ];

    for (ch, qcode) in special_chars {
        map.insert(
            ch,
            KeyMapping {
                keys: KeyPress::full(qcode),
            },
        );
    }

    // Shift + 数字 / 特殊字符
    let shift_chars = vec![
        ('!', "1"),
        ('@', "2"),
        ('#', "3"),
        ('$', "4"),
        ('%', "5"),
        ('^', "6"),
        ('&', "7"),
        ('*', "8"),
        ('(', "9"),
        (')', "0"),
        ('_', "minus"),
        ('+', "equal"),
        ('~', "grave_accent"),
        ('{', "bracket_left"),
        ('}', "bracket_right"),
        ('|', "backslash"),
        (':', "semicolon"),
        ('"', "apostrophe"),
        ('<', "comma"),
        ('>', "dot"),
        ('?', "slash"),
    ];

    for (ch, qcode) in shift_chars {
        map.insert(
            ch,
            KeyMapping {
                keys: vec![
                    KeyPress::press("shift"),
                    KeyPress::press(qcode),
                    KeyPress::release(qcode),
                    KeyPress::release("shift"),
                ],
            },
        );
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_from_str() {
        assert_eq!(KeyboardLayout::from_str("en-us"), Some(KeyboardLayout::EnUS));
        assert_eq!(KeyboardLayout::from_str("US"), Some(KeyboardLayout::EnUS));
        assert_eq!(KeyboardLayout::from_str("unknown"), None);
    }

    #[test]
    fn test_us_layout_map() {
        let map = build_us_layout_map();

        // 测试小写字母
        assert!(map.contains_key(&'a'));

        // 测试大写字母
        let mapping_a = map.get(&'A').unwrap();
        assert_eq!(mapping_a.keys.len(), 4); // shift down, a down, a up, shift up

        // 测试特殊字符
        assert!(map.contains_key(&'@'));
    }
}

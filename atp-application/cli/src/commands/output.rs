//! CLI 通用输出格式化模块
//!
//! 提供 table/json/yaml 三种输出格式的通用实现

use anyhow::Result;
use serde::Serialize;

/// 可输出为表格行的数据 trait
pub trait TableRow {
    /// 返回表格列标题
    fn headers() -> Vec<&'static str>;

    /// 返回该项的表格行数据
    fn row(&self) -> Vec<String>;
}

/// 表格格式输出
pub fn print_table<T: TableRow>(items: &[T], filter: Option<&dyn Fn(&T) -> bool>) {
    let headers = T::headers();

    // 打印表头
    let header_line: String = headers
        .iter()
        .map(|h| format!("{:<20}", h))
        .collect::<Vec<_>>()
        .join(" ");
    println!("{}", header_line);
    println!("{}", "-".repeat(header_line.len()));

    // 打印数据行
    for item in items {
        if let Some(f) = filter {
            if !f(item) {
                continue;
            }
        }

        let row = item.row();
        let row_line: String = row
            .iter()
            .map(|c| format!("{:<20}", c))
            .collect::<Vec<_>>()
            .join(" ");
        println!("{}", row_line);
    }
}

/// JSON 格式输出
pub fn print_json<T: Serialize>(items: &[T]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(items)?);
    Ok(())
}

/// YAML 格式输出
pub fn print_yaml<T: Serialize>(items: &[T]) -> Result<()> {
    for item in items {
        // 简单的 YAML 输出
        let json = serde_json::to_value(item)?;
        if let serde_json::Value::Object(map) = json {
            println!("-");
            for (key, value) in map {
                println!("  {}: {}", key, value);
            }
        }
    }
    Ok(())
}

/// 根据格式参数选择输出方式
pub fn output_formatted<T: TableRow + Serialize>(
    items: &[T],
    format: &str,
    filter: Option<&dyn Fn(&T) -> bool>,
) -> Result<()> {
    match format {
        "json" => {
            let filtered: Vec<_> = if let Some(f) = filter {
                items.iter().filter(|i| f(i)).collect()
            } else {
                items.iter().collect()
            };
            print_json(&filtered)?;
        }
        "yaml" => {
            let filtered: Vec<_> = if let Some(f) = filter {
                items.iter().filter(|i| f(i)).collect()
            } else {
                items.iter().collect()
            };
            print_yaml(&filtered)?;
        }
        _ => print_table(items, filter),
    }
    Ok(())
}

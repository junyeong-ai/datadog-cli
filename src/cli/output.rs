use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::Value;
use std::io::{self, Write};

pub enum Format {
    Json,
    JsonLines,
    Table,
}

impl Format {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Format::Json),
            "jsonl" | "jsonlines" => Ok(Format::JsonLines),
            "table" => Ok(Format::Table),
            _ => Err(format!("Invalid format: {}", s)),
        }
    }
}

pub fn print(data: &Value, format: &Format) -> io::Result<()> {
    match format {
        Format::Json => print_json(data),
        Format::JsonLines => print_jsonlines(data),
        Format::Table => print_table(data),
    }
}

fn print_json(data: &Value) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, data)?;
    writeln!(handle)?;
    Ok(())
}

fn print_jsonlines(data: &Value) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if let Some(items) = data.get("data").and_then(|d| d.as_array()) {
        for item in items {
            serde_json::to_writer(&mut handle, item)?;
            writeln!(handle)?;
        }
    } else if let Some(items) = data.as_array() {
        for item in items {
            serde_json::to_writer(&mut handle, item)?;
            writeln!(handle)?;
        }
    } else {
        serde_json::to_writer(&mut handle, data)?;
        writeln!(handle)?;
    }

    Ok(())
}

fn print_table(data: &Value) -> io::Result<()> {
    let items = if let Some(data_array) = data.get("data").and_then(|d| d.as_array()) {
        data_array
    } else if let Some(array) = data.as_array() {
        array
    } else {
        eprintln!("Error: Data is not in array format for table output");
        return Ok(());
    };

    if items.is_empty() {
        println!("No data");
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    if let Some(first) = items[0].as_object() {
        let headers: Vec<_> = first.keys().collect();
        table.set_header(&headers);

        for item in items {
            if let Some(obj) = item.as_object() {
                let row: Vec<_> = headers.iter().map(|k| format_value(obj.get(*k))).collect();
                table.add_row(row);
            }
        }
    }

    println!("{table}");
    Ok(())
}

fn format_value(value: Option<&Value>) -> String {
    match value {
        None => "-".to_string(),
        Some(Value::Null) => "-".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                format!("[{} items]", arr.len())
            }
        }
        Some(Value::Object(_)) => "{...}".to_string(),
    }
}

use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::Value;
use std::io::{self, IsTerminal, Write};

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
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    write(data, format, &mut writer, stdout.is_terminal())?;
    writer.flush()
}

pub fn print_scalar(data: &Value, format: &Format) -> io::Result<()> {
    if !matches!(format, Format::Table) {
        return print(data, format);
    }
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    write_scalar_table(data, &mut writer)?;
    writer.flush()
}

fn write_scalar_table(data: &Value, writer: &mut impl Write) -> io::Result<()> {
    let columns = data["data"]["columns"].as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Scalar output requires columns",
        )
    })?;
    let rows = columns
        .first()
        .and_then(|column| column["values"].as_array())
        .map_or(0, Vec::len);
    if rows == 0 {
        return writeln!(writer, "No data");
    }
    let mut table = Table::new();
    table.load_style(UTF8_FULL);
    table.set_header(columns.iter().enumerate().map(|(index, column)| {
        format!(
            "[{index}] {} ({})",
            column["name"].as_str().unwrap_or("unnamed"),
            column["type"].as_str().unwrap_or("unknown")
        )
    }));
    for row in 0..rows {
        table.add_row(columns.iter().map(|column| {
            let value = &column["values"][row];
            if value.is_null() {
                "-".into()
            } else {
                value.to_string()
            }
        }));
    }
    writeln!(writer, "{table}")
}

pub fn write(
    data: &Value,
    format: &Format,
    writer: &mut impl Write,
    pretty: bool,
) -> io::Result<()> {
    match format {
        Format::Json => {
            if pretty {
                serde_json::to_writer_pretty(&mut *writer, data)?;
            } else {
                serde_json::to_writer(&mut *writer, data)?;
            }
            writeln!(writer)
        }
        Format::JsonLines => {
            let data = data.get("data").unwrap_or(data);
            if let Some(items) = data.as_array() {
                for item in items {
                    serde_json::to_writer(&mut *writer, item)?;
                    writeln!(writer)?;
                }
            } else {
                serde_json::to_writer(&mut *writer, data)?;
                writeln!(writer)?;
            }
            Ok(())
        }
        Format::Table => write_table(data, writer),
    }
}

fn write_table(data: &Value, writer: &mut impl Write) -> io::Result<()> {
    let data = data.get("data").unwrap_or(data);
    let items: Vec<&Value> = match data {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![data],
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Table output requires an object or an array of objects",
            ));
        }
    };
    if items.is_empty() {
        return writeln!(writer, "No data");
    }
    if items.iter().any(|item| !item.is_object()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Table rows must be objects",
        ));
    }

    let headers: std::collections::BTreeSet<&String> = items
        .iter()
        .filter_map(|item| item.as_object())
        .flat_map(|item| item.keys())
        .collect();
    let mut table = Table::new();
    table.load_style(UTF8_FULL);
    table.set_header(&headers);
    for item in items {
        table.add_row(
            headers
                .iter()
                .map(|key| format_value(item.get(*key)))
                .collect::<Vec<_>>(),
        );
    }
    writeln!(writer, "{table}")
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jsonlines_preserves_records_without_metadata() {
        let mut output = Vec::new();
        write(
            &json!({"data": [{"id": 1}, {"id": 2}], "pagination": {"next_cursor": "next"}}),
            &Format::JsonLines,
            &mut output,
            false,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"id\":1}\n{\"id\":2}\n"
        );
    }

    #[test]
    fn table_supports_details_and_sparse_rows() {
        for data in [
            json!({"data": {"id": 1}}),
            json!({"data": [{"a": 1}, {"b": 2}]}),
        ] {
            let mut output = Vec::new();
            write(&data, &Format::Table, &mut output, false).unwrap();
            assert!(!output.is_empty());
        }
        assert!(write(&json!([1]), &Format::Table, &mut Vec::new(), false).is_err());
    }

    #[test]
    fn every_format_propagates_broken_pipe() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        for format in [Format::Json, Format::JsonLines, Format::Table] {
            let error =
                write(&json!({"data": [{"id": 1}]}), &format, &mut Closed, false).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        }
    }

    #[test]
    fn scalar_formats_preserve_duplicate_columns_and_values() {
        let data = json!({"data": {"columns": [
            {"name": "a", "type": "group", "values": [["prod"]]},
            {"name": "a", "type": "number", "values": [1.25]},
            {"type": "number", "values": [null]}
        ]}});
        let mut table = Vec::new();
        write_scalar_table(&data, &mut table).unwrap();
        let table = String::from_utf8(table).unwrap();
        for text in [
            "[0] a (group)",
            "[1] a (number)",
            "[2] unnamed (number)",
            "prod",
            "1.25",
        ] {
            assert!(table.contains(text));
        }
        let mut jsonl = Vec::new();
        write(&data, &Format::JsonLines, &mut jsonl, false).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&jsonl).unwrap(),
            data["data"]
        );
        assert_eq!(jsonl.iter().filter(|&&byte| byte == b'\n').count(), 1);
    }

    #[test]
    fn jsonlines_details_omit_the_envelope() {
        let mut output = Vec::new();
        write(
            &json!({"data": {"id": 1}, "meta": {"query": "q"}}),
            &Format::JsonLines,
            &mut output,
            false,
        )
        .unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "{\"id\":1}\n");
    }
}

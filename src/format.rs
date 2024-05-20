use core::fmt;
use core::fmt::{Display, Result};
use urlencoding::decode;

use serde_json::{Map, Value};

use crate::prefix::{Prefix, MAX_LEVEL};

const INDENT_WIDTH: usize = 2;

struct Wrapper<'a>(&'a Value);

impl<'a> Display for Wrapper<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        match &self.0 {
            Value::String(string) => write!(f, "{}", string),
            Value::Number(number) => write!(f, "{}", number),
            Value::Bool(boolean) => write!(f, "{}", boolean),
            _ => Ok(()),
        }
    }
}

fn format_map(f: &mut fmt::Formatter<'_>, indent: usize, map: &Map<String, Value>) -> Result {
    for (key, value) in map {
        match value {
            Value::Null => writeln!(f, "{:length$}{}", "", key, length = indent)?,
            Value::Bool(boolean) => {
                let value = ["no", "yes"][*boolean as usize];
                writeln!(f, "{:length$}{} {}", "", key, value, length = indent)?
            }
            Value::Number(number) => {
                writeln!(f, "{:length$}{} {}", "", key, number, length = indent)?
            }
            Value::String(string) => {
                writeln!(f, "{:length$}{} {}", "", key, string, length = indent)?
            }
            Value::Array(array) => {
                for item in array {
                    writeln!(f, "{:length$}{} {}", "", key, Wrapper(item), length = indent)?;
                }
            }
            Value::Object(map) => {
                writeln!(f, "{:length$}{}", "", key, length = indent)?;
                format_map(f, indent + INDENT_WIDTH, map)?;
            }
        }
    }
    Ok(())
}

fn format_entry(f: &mut fmt::Formatter<'_>, indent: usize, key: &str, value: &Value) -> Result {
    match value {
        Value::Null => {
            writeln!(f, "{:length$}{}", "", key, length = indent)
        }
        Value::Array(array) => {
            for item in array {
                writeln!(f, "{:length$}{} {}", "", key, Wrapper(item), length = indent)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            if key != "" {
                writeln!(f, "{:length$}{}", "", key, length = indent)?;
            }
            format_map(f, indent + INDENT_WIDTH, map)
        }
        _ => Ok(()),
    }
}

fn decode_path(path: &str) -> String {
    if path.split('/').any(|chunk| decode(chunk).unwrap_or_default().contains('/')) {
        return decode(&path.replace('/', ".")).unwrap_or_default().to_string();
    }
    decode(path).unwrap_or_default().to_string()
}

pub struct Formatter<'a, S: AsRef<str>>(pub &'a [(S, Value)]);

impl<'a, S: AsRef<str>> Display for Formatter<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        let prefixes = Prefix::build(self.0.iter().map(|(key, _)| key.as_ref()));
        let mut current = heapless::Vec::<&Prefix, MAX_LEVEL>::new();
        let mut index = 0;
        let mut indent = 0;
        let mut prefix_len = 0;
        for (i, entry) in self.0.iter().enumerate() {
            while i >= current.last().map(|p| p.range.end).unwrap_or(usize::MAX) {
                let pop = current.pop().unwrap();
                prefix_len -= pop.text.len();
                indent -= INDENT_WIDTH;
            }
            while index < prefixes.len() && i >= prefixes[index].range.start {
                let prefix = &prefixes[index];
                prefix_len += prefix.text.len();
                let mut prefix_text = decode_path(&prefix.text);
                if prefix_len == entry.0.as_ref().len() {
                    prefix_text.push(':');
                }
                writeln!(f, "{:length$}{}", "", prefix_text, length = indent)?;
                current.push(prefix).unwrap();
                index += 1;
                indent += INDENT_WIDTH;
            }
            if entry.0.as_ref().len() == prefix_len {
                format_entry(f, indent - INDENT_WIDTH, "", &entry.1)?;
                continue;
            }
            let path = entry.0.as_ref()[prefix_len..].to_owned() + ":";
            format_entry(f, indent, &decode_path(&path), &entry.1)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_format() {
        let test_data = include_str!("../test/sample-data.yaml");
        let data = match serde_yaml::from_str(test_data).unwrap() {
            serde_json::Value::Object(map) => map,
            _ => panic!("Not a mapping"),
        };
        let mut entries: Vec<(String, serde_json::Value)> = data.into_iter().collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let output = format!("{}", super::Formatter(&entries));
        assert_eq!(include_str!("../test/sample-output.txt"), output);
    }
}

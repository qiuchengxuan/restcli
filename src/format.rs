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

#[derive(Copy, Clone)]
struct Context<'a> {
    indent_width: usize,
    yesno: [&'static str; 2],
    key: &'a str,
    indent: usize,
}

impl Default for Context<'static> {
    fn default() -> Self {
        Self { indent_width: 2, yesno: ["yes", "no"], indent: 0, key: "" }
    }
}

trait Format {
    fn format<'a>(&self, f: &mut fmt::Formatter<'_>, ctx: Context<'a>) -> Result;
}

impl Format for Map<String, Value> {
    fn format<'a>(&self, f: &mut fmt::Formatter<'_>, ctx: Context<'a>) -> Result {
        for (key, value) in self {
            match value {
                Value::Null => writeln!(f, "{:indent$}{}", "", key, indent = ctx.indent)?,
                Value::Bool(boolean) => {
                    let value = ctx.yesno[*boolean as usize];
                    writeln!(f, "{:indent$}{} {}", "", key, value, indent = ctx.indent)?
                }
                Value::Number(number) => {
                    writeln!(f, "{:indent$}{} {}", "", key, number, indent = ctx.indent)?
                }
                Value::String(string) => {
                    writeln!(f, "{:indent$}{} {}", "", key, string, indent = ctx.indent)?
                }
                Value::Array(array) => {
                    let indent = ctx.indent;
                    for item in array {
                        writeln!(f, "{:indent$}{} {}", "", key, Wrapper(item), indent = indent)?;
                    }
                }
                Value::Object(map) => {
                    writeln!(f, "{:indent$}{}", "", key, indent = ctx.indent)?;
                    let ctx = Context { indent: ctx.indent + ctx.indent_width, key: "", ..ctx };
                    map.format(f, ctx)?;
                }
            }
        }
        Ok(())
    }
}

impl Format for Value {
    fn format<'a>(&self, f: &mut fmt::Formatter<'_>, ctx: Context<'a>) -> Result {
        match self {
            Value::Null => {
                writeln!(f, "{:indent$}{}", "", ctx.key, indent = ctx.indent)
            }
            Value::Array(array) => {
                let indent = ctx.indent;
                for item in array {
                    writeln!(f, "{:indent$}{} {}", "", ctx.key, Wrapper(item), indent = indent)?;
                }
                Ok(())
            }
            Value::Object(map) => {
                if ctx.key != "" {
                    writeln!(f, "{:indent$}{}", "", ctx.key, indent = ctx.indent)?;
                }
                map.format(f, Context { indent: ctx.indent + ctx.indent_width, ..ctx })
            }
            _ => Ok(()),
        }
    }
}

fn decode_path(path: &str) -> String {
    if path.split('/').any(|chunk| decode(chunk).unwrap_or_default().contains('/')) {
        return decode(&path.replace('/', ".")).unwrap_or_default().to_string();
    }
    decode(path).unwrap_or_default().to_string()
}

pub struct Formatter<'a, S: AsRef<str>> {
    records: &'a [(S, Value)],
    yesno: [&'static str; 2],
    indent_width: usize,
}

impl<'a, S: AsRef<str>> From<&'a [(S, Value)]> for Formatter<'a, S> {
    fn from(records: &'a [(S, Value)]) -> Self {
        Self { records, yesno: ["yes", "no"], indent_width: 2 }
    }
}

impl<'a, S: AsRef<str>> Display for Formatter<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        let prefixes = Prefix::build(self.records.iter().map(|(key, _)| key.as_ref()));
        let mut current = heapless::Vec::<&Prefix, MAX_LEVEL>::new();
        let mut index = 0;
        let mut ctx =
            Context { indent_width: self.indent_width, yesno: self.yesno, ..Default::default() };
        let mut prefix_len = 0;
        for (i, record) in self.records.iter().enumerate() {
            while i >= current.last().map(|p| p.range.end).unwrap_or(usize::MAX) {
                let pop = current.pop().unwrap();
                prefix_len -= pop.text.len();
                ctx.indent -= self.indent_width;
            }
            while index < prefixes.len() && i >= prefixes[index].range.start {
                let prefix = &prefixes[index];
                prefix_len += prefix.text.len();
                let mut prefix_text = decode_path(&prefix.text);
                if prefix_len == record.0.as_ref().len() {
                    prefix_text.push(':');
                }
                writeln!(f, "{:indent$}{}", "", prefix_text, indent = ctx.indent)?;
                current.push(prefix).unwrap();
                index += 1;
                ctx.indent += INDENT_WIDTH;
            }
            if record.0.as_ref().len() == prefix_len {
                record.1.format(f, Context { indent: ctx.indent - self.indent_width, ..ctx })?;
                continue;
            }
            let path = record.0.as_ref()[prefix_len..].to_owned() + ":";
            record.1.format(f, Context { key: &decode_path(&path), ..ctx })?;
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
        let output = format!("{}", super::Formatter::from(entries.as_slice()));
        assert_eq!(include_str!("../test/sample-output.txt"), output);
    }
}

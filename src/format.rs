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

type KeywordsFn = fn(&str) -> Option<&str>;

#[derive(Copy, Clone)]
struct Context<'a> {
    indent_width: usize,
    yesno: [&'static str; 2],
    keywords: KeywordsFn,
    key: &'a str,
    indent: usize,
}

impl Default for Context<'static> {
    fn default() -> Self {
        Self { indent_width: 2, yesno: ["yes", "no"], keywords: |_| None, indent: 0, key: "" }
    }
}

impl<'a> Context<'a> {
    fn new(indent_width: usize, yesno: [&'static str; 2], keywords: KeywordsFn) -> Self {
        Self { indent_width, yesno, keywords, ..Default::default() }
    }
}

trait IsPrimitive {
    fn is_primitive(&self) -> bool;
}

impl IsPrimitive for Value {
    fn is_primitive(&self) -> bool {
        match self {
            Value::Array(_) | Value::Object(_) => false,
            _ => true,
        }
    }
}

trait Format {
    fn format<'a>(&self, f: &mut fmt::Formatter<'_>, ctx: Context<'a>) -> Result;
}

impl Format for Vec<Value> {
    fn format<'a>(&self, f: &mut fmt::Formatter<'_>, ctx: Context<'a>) -> Result {
        if self.len() == 0 {
            return Ok(());
        }
        if self.as_slice().iter().all(|v| v.is_primitive()) {
            let key = (ctx.keywords)(ctx.key).unwrap_or(ctx.key);
            for item in self {
                writeln!(f, "{:indent$}{} {}", "", key, Wrapper(item), indent = ctx.indent)?;
            }
            return Ok(());
        }
        writeln!(f, "{:indent$}{}", "", ctx.key, indent = ctx.indent)?;
        let indent = ctx.indent + ctx.indent_width;
        for item in self {
            writeln!(f, "{:indent$}!", "", indent = indent)?;
            writeln!(f, "{:indent$}{}", "", Wrapper(item), indent = indent)?;
        }
        writeln!(f, "{:indent$}!", "", indent = indent)
    }
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
                Value::Array(array) => array.format(f, Context { key, ..ctx })?,
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
    keywords: fn(&str) -> Option<&str>,
}

impl<'a, S: AsRef<str>> Formatter<'a, S> {
    pub fn new(records: &'a [(S, Value)], keywords: KeywordsFn) -> Self {
        Self { records, yesno: ["yes", "no"], indent_width: 2, keywords }
    }
}

impl<'a, S: AsRef<str>> Display for Formatter<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        let prefixes = Prefix::build(self.records.iter().map(|(key, _)| key.as_ref()));
        let mut current = heapless::Vec::<&Prefix, MAX_LEVEL>::new();
        let mut index = 0;
        let mut ctx = Context::new(self.indent_width, self.yesno, self.keywords);
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
    fn keywords(word: &str) -> Option<&str> {
        Some(word.trim_end_matches('s'))
    }

    #[test]
    fn test_format() {
        let test_data = include_str!("../test/sample-data.yaml");
        let data = match serde_yaml::from_str(test_data).unwrap() {
            serde_json::Value::Object(map) => map,
            _ => panic!("Not a mapping"),
        };
        let mut entries: Vec<(String, serde_json::Value)> = data.into_iter().collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let output = format!("{}", super::Formatter::new(entries.as_slice(), keywords));
        assert_eq!(include_str!("../test/sample-output.txt"), output);
    }
}

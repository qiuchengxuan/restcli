use std::cmp::Ordering;
use std::io;
use std::io::Write;

use request::header::HeaderMap;
use request::header::HeaderValue;
use request::header::ACCEPT;
use serde_json::Value;
use termion::event;
use termion::input::TermRead;

use crate::config::API;
use crate::format::Formatter;

struct Rest {
    url: String,
    headers: HeaderMap,
}

impl Rest {
    fn get(&self, path: &str) -> request::Result<Value> {
        let url = self.url.clone() + path.trim_start_matches('/');
        let client = request::blocking::Client::new();
        client.get(url).headers(self.headers.clone()).send()?.json()
    }
}

struct Querier<'a> {
    rest: &'a Rest,
    apis: &'a [API],
    filter: &'a str,
    more: bool,
    root: Option<String>,
    results: Vec<(String, Value)>,
}

impl<'a> Querier<'a> {
    fn query_apis(&mut self, apis: &[API], prefix: String) -> request::Result<()> {
        let mut more = false;
        for api in apis {
            let path = prefix.clone() + api.path.trim_start_matches('/');
            let mut value = self.rest.get(&path)?;
            if let Some(jsonpath) = api.jsonpath.as_ref() {
                value = jsonpath::find(&jsonpath.0, &value);
            }
            let records = match value {
                Value::Object(object) => object,
                _ => continue,
            };
            trace!("Found {} records", records.len());
            let sub_apis = api.apis.as_ref().map(|v| v.as_slice()).unwrap_or_default();
            for (key, value) in records.into_iter() {
                let path = prefix.clone() + key.trim_matches('/');
                self.results.push((path.clone(), value));
                if sub_apis.is_empty() {
                    continue;
                }
                more = true;
                if api.is_entity != Some(true) || self.filter.starts_with(&path) {
                    self.query_apis(sub_apis, path + "/")?;
                }
            }
        }
        if self.root.is_none() {
            (self.more, self.root) = (more, Some(prefix));
        }
        Ok(())
    }

    fn query(mut self) -> request::Result<(bool, String, Vec<(String, Value)>)> {
        self.query_apis(self.apis, "/".into()).map(|_| Default::default())?;
        self.results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok((self.more, self.root.unwrap_or("/".into()), self.results))
    }

    fn new(rest: &'a Rest, apis: &'a [API], filter: &'a str) -> Self {
        Self { rest, apis, filter, more: false, root: None, results: Vec::new() }
    }
}

pub struct CLI {
    rest: Rest,
    apis: Vec<API>,
    more: bool,
    root: String,
    records: Vec<(String, Value)>,
    current_path: String,
}

impl CLI {
    pub fn new(url: String, apis: Vec<API>) -> request::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let rest = Rest { url, headers };
        let (more, root, records) = Querier::new(&rest, &apis, "/").query()?;
        Ok(Self { rest, apis, more, root, records, current_path: "/".into() })
    }

    fn filter_records<'a>(&'a self) -> &'a [(String, Value)] {
        if self.current_path == "/" {
            return &self.records;
        }
        let path = &self.current_path;
        let start = self.records.binary_search_by(|(key, _)| key.cmp(path)).unwrap_or_else(|e| e);
        let end = self.records[start..].binary_search_by(|(key, _)| match key.starts_with(path) {
            true => Ordering::Less,
            false => key.cmp(path),
        });
        return &self.records[start..start + end.unwrap_or_else(|e| e)];
    }

    fn refresh(&mut self) -> request::Result<()> {
        let (rest, apis, path) = (&self.rest, &self.apis, &self.current_path);
        (self.more, self.root, self.records) = Querier::new(rest, apis, path).query()?;
        trace!("Root {} more {}", self.root, self.more);
        Ok(())
    }

    fn change_directory(&mut self, arg: &str) {
        let (truncate, append) = match arg {
            ".." => match self.current_path.trim_end_matches('/').rsplit_once('/') {
                Some((left, _)) => (left.len(), ""),
                None => (0, ""),
            },
            _ if arg.starts_with('/') => (0, arg),
            _ => (self.current_path.len(), arg),
        };
        if truncate == self.current_path.len() && append == "" {
            return;
        }
        let mut prefix = self.current_path[..truncate].to_owned();
        if append != "" {
            if !append.starts_with('/') && !prefix.ends_with('/') {
                prefix.push('/');
            }
            prefix += append;
        }
        if let Some(index) = self.records.binary_search_by(|(key, _)| key.cmp(&prefix)).err() {
            if !prefix.ends_with('/') {
                prefix.push('/');
            }
            if index >= self.records.len() || !self.records[index].0.starts_with(&prefix) {
                println!("No such path");
                return;
            }
        }
        self.current_path = prefix;
        if append != "" && !self.more {
            return;
        }
        if append == "" && self.current_path.starts_with(&self.root) {
            return;
        }
        if let Some(err) = self.refresh().err() {
            eprintln!("Request backend failed: {}", err)
        }
    }

    pub fn run(&mut self) {
        let mut buf = Vec::new();
        print!("restcli {}> ", self.current_path);
        io::stdout().flush().unwrap();
        for event in io::stdin().events() {
            let bytes = match event.unwrap() {
                event::Event::Key(event::Key::Char(ch)) => {
                    let mut buf = [0; 4];
                    ch.encode_utf8(&mut buf).as_bytes().to_vec()
                }
                _ => continue,
            };
            for byte in bytes {
                if byte != b'\n' {
                    buf.push(byte);
                    continue;
                }
                let line = std::str::from_utf8(buf.as_slice()).unwrap_or_default();
                let (command, arg) = line.split_once(' ').unwrap_or((line, ""));
                match command {
                    "cd" => self.change_directory(arg),
                    "list" => {
                        println!("{}", Formatter::from(self.filter_records()))
                    }
                    "exit" => return,
                    line => eprintln!("Unknown command {}", line),
                }
                buf.clear();
                print!("restcli {}> ", self.current_path);
                io::stdout().flush().unwrap();
            }
        }
    }
}

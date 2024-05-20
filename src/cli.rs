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
    results: Vec<(String, Value)>,
}

impl<'a> Querier<'a> {
    fn query_sub(&mut self, sub_apis: &[API], prefix: String) -> request::Result<()> {
        for api in sub_apis {
            let sub_paths = api.sub_apis.as_ref().map(|v| v.as_slice()).unwrap_or_default();
            let path = prefix.clone() + &api.path;
            trace!("GET {}", path);
            let value = self.rest.get(&path)?;
            self.with_value(&path, value, sub_paths)?
        }
        Ok(())
    }

    fn with_value(&mut self, prefix: &str, value: Value, sub: &[API]) -> request::Result<()> {
        match value {
            Value::Object(object) => {
                for (key, value) in object.into_iter() {
                    let path = prefix.to_owned() + &key;
                    self.results.push((path.clone(), value));
                    self.query_sub(sub, path)?;
                }
            }
            _ => return Ok(()),
        }
        Ok(())
    }

    fn query(mut self) -> request::Result<Vec<(String, Value)>> {
        for api in self.apis {
            trace!("GET {}", api.path);
            let mut value = self.rest.get(&api.path)?;
            if let Some(jsonpath) = api.jsonpath.as_ref() {
                value = jsonpath::find(&jsonpath.0, &value);
            }
            let sub_apis = api.sub_apis.as_ref().map(|v| v.as_slice()).unwrap_or_default();
            self.with_value(&api.path, value, sub_apis)?
        }
        self.results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(self.results)
    }
}

pub struct CLI {
    rest: Rest,
    apis: Vec<API>,
}

impl CLI {
    pub fn new(url: String, apis: Vec<API>) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let rest = Rest { url, headers };
        Self { rest, apis }
    }

    fn list(&self) -> request::Result<()> {
        let querier = Querier { rest: &self.rest, apis: &self.apis, results: Vec::new() };
        println!("{}", Formatter(&querier.query()?));
        Ok(())
    }

    pub fn run(&self) -> Result<(), String> {
        let mut buf = Vec::new();
        print!("restcli> ");
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
                match std::str::from_utf8(buf.as_slice()).unwrap_or_default() {
                    "list" => {
                        if let Some(err) = self.list().err() {
                            eprintln!("Request backend failed: {}", err)
                        }
                    }
                    "exit" => return Ok(()),
                    line => eprintln!("Unknown command {}", line),
                }
                buf.clear();
                print!("restcli> ");
                io::stdout().flush().unwrap();
            }
        }
        Ok(())
    }
}

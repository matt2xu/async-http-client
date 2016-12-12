//! Definition of structures.

use std::ops::Index;

#[derive(PartialEq, Eq, Debug)]
pub struct Header {
    name: String,
    value: Option<String>
}

impl Header {
    pub fn new<K: Into<String>, V: Into<String>>(name: K, value: V) -> Header {
        Header {
            name: name.into(),
            value: Some(value.into())
        }
    }
}


#[derive(PartialEq, Eq, Debug)]
pub struct HttpResponse {
    version: (u32, u32),
    status: u32,
    headers: Vec<Header>,
    body: Vec<u8>
}

impl HttpResponse {
    pub fn new(version: (u32, u32), status: u32, headers: Vec<Header>) -> HttpResponse {
        HttpResponse {
            version: version,
            status: status,
            headers: headers,
            body: Vec::new()
        }
    }

    pub fn status(&self) -> u32 {
        self.status
    }

    pub fn is_informational(&self) -> bool {
        self.status >= 100 && self.status < 200
    }

    pub fn is_successful(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn is_redirection(&self) -> bool {
        self.status >= 300 && self.status < 400
    }

    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }

    pub fn append<A: AsRef<[u8]>>(&mut self, buf: A) {
        self.body.extend_from_slice(buf.as_ref());
    }

    fn get(&self, name: &str) -> Option<&Option<String>> {
        self.headers.iter().find(|header| header.name == name).map(|header| &header.value)
    }
}

const NONE: &'static Option<String> = &None;

impl<'a> Index<&'a str> for HttpResponse {
    type Output = Option<String>;

    fn index(&self, name: &str) -> &Option<String> {
        self.get(name).unwrap_or(NONE)
    }
}

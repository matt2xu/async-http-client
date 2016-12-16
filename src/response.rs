//! Definition of response structure.

use std::ascii::AsciiExt;
use std::cmp;
use std::fmt;
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

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value.as_ref().map(|s| s.as_str()).unwrap_or(""))
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

    /// Returns true if this response has a header with the given name
    /// that matches the expected value.
    ///
    /// Comparisons are made in a case-insensitive manner.
    pub fn has<K: AsRef<str>, V: AsRef<str>>(&self, name: K, expected: V) -> bool {
        self[name.as_ref()].as_ref()
            .map_or(false, |candidate| expected.as_ref().eq_ignore_ascii_case(&candidate))
    }

    /// Returns true if this response has a 1xx Informational status code.
    pub fn is_informational(&self) -> bool {
        self.status >= 100 && self.status < 200
    }

    /// Returns true if this response has a 2xx Successful status code.
    pub fn is_successful(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Returns true if this response has a 3xx Redirection status code.
    pub fn is_redirection(&self) -> bool {
        self.status >= 300 && self.status < 400
    }

    /// Returns true if this response has a 4xx Client Error status code.
    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    /// Returns true if this response isisis a 5xx Server Error status code.
    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }
}

/// Appends data to this response's body.
pub fn append<A: AsRef<[u8]>>(res: &mut HttpResponse, buf: A) {
    res.body.extend_from_slice(buf.as_ref());
}

const NONE: &'static Option<String> = &None;

impl<'a> Index<&'a str> for HttpResponse {
    type Output = Option<String>;

    /// Retrieve the header with the given name.
    ///
    /// Comparison is made in a case-insensitive manner.
    fn index(&self, name: &str) -> &Option<String> {
        self.headers.iter()
            .find(|header| name.eq_ignore_ascii_case(&header.name))
            .map(|header| &header.value).unwrap_or(NONE)
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "HTTP/{}.{} {}", self.version.0, self.version.1, self.status)?;
        for header in &self.headers {
            writeln!(f, "{}", header)?;
        }
        write!(f, "body: {} bytes = [", self.body.len())?;
        for byte in &self.body[0 .. cmp::min(self.body.len(), 30)] {
            write!(f, "{}", *byte as char)?;
        }
        writeln!(f, "...]")
    }
}

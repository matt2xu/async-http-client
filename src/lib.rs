extern crate futures;
extern crate tokio_core;
extern crate url;

use std::borrow::Cow;
use std::fmt;
use std::io;
use std::net::ToSocketAddrs;

use futures::Future;

use tokio_core::io::{copy, write_all};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;

use url::{Url, ParseError};

pub struct HttpRequest<'a> {
    url: Url,
    method: Method,
    headers: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    body: Option<&'a [u8]>
}

pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Other(String)
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Method::*;
        match *self {
            Get => write!(f, "GET"),
            Head => write!(f, "HEAD"),
            Post => write!(f, "POST"),
            Put => write!(f, "PUT"),
            Delete => write!(f, "DELETE"),
            Connect => write!(f, "CONNECT"),
            Options => write!(f, "OPTIONS"),
            Trace => write!(f, "TRACE"),
            Other(ref other) => write!(f, "{}", other)
        }
    }
}

impl<'a> HttpRequest<'a> {
    pub fn new<S: AsRef<str>>(url: S) -> Result<HttpRequest<'a>, ParseError> {
        url.as_ref().parse().map(|url: Url| {
            use std::fmt::Write;

            let mut host = url.host_str().unwrap_or("").to_string();
            if let Some(port) = url.port() {
                write!(host, ":{}", port).unwrap();
            }

            HttpRequest {
                url: url,
                method: Method::Get,
                headers: vec![],
                body: None
            }.header("Host", host)
        })
    }

    pub fn header<I: Into<Cow<'a, str>>>(mut self, name: &'a str, value: I) -> HttpRequest<'a> {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn post(self, body: &'a [u8]) -> HttpRequest<'a> {
        let mut req = self.header("Content-Length", body.len().to_string());
        req.method = Method::Post;
        req.body = Some(body);
        req
    }

    pub fn send(&mut self, core: &mut Core) -> Result<(), io::Error> {
        let request = self.to_string();
        println!("{}", request);

        let mut addrs = self.url.to_socket_addrs()?;
        let addr = addrs.next().unwrap();

        let handle = core.handle();
        let future = TcpStream::connect(&addr, &handle).and_then(|stream| {
            write_all(stream, request.as_bytes()).and_then(|(stream, _bytes)| {
                write_all(stream, self.body.unwrap_or(&[]))
            }).and_then(|(stream, _bytes)| {
                copy(stream, io::stdout())
            }).map(|_| ())
        });

        core.run(future)
    }
}


impl<'a> fmt::Display for HttpRequest<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // request line
        write!(f, "{} {}", self.method, self.url.path())?;
        if let Some(query) = self.url.query() {
            write!(f, "?{}", query)?;
        }
        if let Some(fragment) = self.url.fragment() {
            write!(f, "#{}", fragment)?;
        }
        write!(f, " HTTP/1.1\r\n")?;

        // headers
        for &(ref name, ref value) in &self.headers {
            write!(f, "{}: {}\r\n", name, value)?;
        }
        write!(f, "\r\n")
    }
}

#[cfg(test)]
mod tests {
    use tokio_core::reactor::Core;

    use HttpRequest;

    #[test]
    fn it_works() {
        // Create the event loop that will drive this server
        let mut core = Core::new().unwrap();
        let string = "http://localhost:3000/segment/chunks".to_string();
        let req = HttpRequest::new(&string).unwrap();
        req.header("Content-Type", "text/plain")
            .header("Connection", "Close")
            .post(&[1, 2, 3, 4])
            .send(&mut core).unwrap();
    }
}

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
use tokio_core::reactor::{Core, Handle};

use url::{Url, ParseError};

pub struct HttpRequest {
    url: Url,
    method: Method,
    headers: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    body: Vec<u8>
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

impl HttpRequest {
    pub fn new<U: AsRef<str>>(url: U) -> Result<HttpRequest, ParseError> {
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
                body: vec![]
            }.header("Host", host)
        })
    }

    pub fn header<K: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>>(mut self, name: K, value: V) -> HttpRequest {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn post<U: AsRef<str>, I: Into<Vec<u8>>>(url: U, body: I) -> Result<HttpRequest, ParseError> {
        let bytes = body.into();
        let mut req = Self::new(url)?.header("Content-Length", bytes.len().to_string());
        req.method = Method::Post;
        req.body = bytes;
        Ok(req)
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
}

impl fmt::Display for HttpRequest {
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

pub struct HttpClient {
    stream: TcpStream
}

impl HttpClient {
    pub fn new(core: &mut Core, url: &Url) -> Result<HttpClient, io::Error> {
        let mut addrs = url.to_socket_addrs()?;
        let addr = addrs.next().unwrap();
        let handle = core.handle();
        Ok(HttpClient {
           stream: core.run(TcpStream::connect(&addr, &handle))?
        })
    }

    pub fn send(&mut self, core: &mut Core, req: HttpRequest) -> Result<(), io::Error> {
        let request = req.to_string();
        println!("{}", request);


        let future = write_all(&self.stream, request.as_bytes()).and_then(|(stream, _bytes)| {
                write_all(stream, &req.body)
            }).and_then(|(stream, _bytes)| {
                copy(stream, io::stdout())
            }).map(|_| ());

        core.run(future)
    }
}

#[cfg(test)]
mod tests {
    use tokio_core::reactor::Core;

    use {HttpRequest, HttpClient};

    #[test]
    fn it_works() {
        // Create the event loop that will drive this server
        let string = "http://localhost:3000/segment/chunks".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4]).unwrap()
            .header("Content-Type", "text/plain")
            .header("Connection", "Close");

        let mut core = Core::new().unwrap();
        HttpClient::new(&mut core, req.url()).map(|mut client| {
            client.send(&mut core, req)
        }).unwrap();
    }
}

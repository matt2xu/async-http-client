extern crate futures;
extern crate tokio_core;
extern crate url;

#[macro_use]
extern crate nom;

use std::borrow::Cow;
use std::fmt;
use std::io::{self, Error, ErrorKind, Write};
use std::net::{SocketAddr, ToSocketAddrs};

use futures::{Future, Sink, Stream};

use tokio_core::io::{EasyBuf, Codec, Framed, Io, IoFuture};

pub mod prelude {
    pub use tokio_core::io::Io;
    pub use tokio_core::net::TcpStream;
    pub use tokio_core::reactor::Core;
    pub use futures::{Future, Sink, Stream};
}

pub use futures::sync::mpsc;

use url::{Url, ParseError};

use nom::IResult;

pub mod parser;
mod structs;

pub use structs::{HttpResponse, Header};

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
    pub fn new<U: AsRef<str>>(method: Method, url: U) -> Result<HttpRequest, ParseError> {
        url.as_ref().parse().map(|url: Url| {
            use std::fmt::Write;

            let mut host = url.host_str().unwrap_or("").to_string();
            if let Some(port) = url.port() {
                write!(host, ":{}", port).unwrap();
            }

            HttpRequest {
                url: url,
                method: method,
                headers: vec![],
                body: vec![]
            }.header("Host", host)
        })
    }

    pub fn header<K: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>>(mut self, name: K, value: V) -> HttpRequest {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn get<U: AsRef<str>>(url: U) -> Result<HttpRequest, ParseError> {
        Self::new(Method::Get, url)
    }

    pub fn post<U: AsRef<str>, I: Into<Vec<u8>>>(url: U, body: I) -> Result<HttpRequest, ParseError> {
        let bytes = body.into();
        let mut req = Self::new(Method::Post, url)?.header("Content-Length", bytes.len().to_string());
        req.body = bytes;
        Ok(req)
    }

    pub fn addr(&self) -> Result<SocketAddr, Error> {
        let mut addrs = self.url.to_socket_addrs()?;
        addrs.next().ok_or(Error::new(ErrorKind::UnexpectedEof, "no address"))
    }

    /// Returns a future that, given a framed, will resolve to a tuple (response?, framed).
    pub fn send<T: 'static + Io + Send>(self, framed: Framed<T, HttpCodec>) -> IoFuture<(Option<HttpResponse>, Framed<T, HttpCodec>)> {
        framed.send(self).and_then(|framed| {
            framed.into_future().and_then(|(res, stream)| {
                println!("for each {:?}", res);
                Ok((res, stream))
            }).map_err(|(err, _stream)| err)
        }).boxed()
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

#[derive(Debug)]
enum CodecState {
    ParsingHeaders,
    ReadingBody(HttpResponse)
}

#[derive(Debug)]
pub struct HttpCodec {
    state: CodecState
}

impl HttpCodec {
    pub fn new() -> HttpCodec {
        HttpCodec {state: CodecState::ParsingHeaders}
    }
}

impl Codec for HttpCodec {
    type In = HttpResponse;
    type Out = HttpRequest;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<HttpResponse>, Error> {
        let num_bytes_total = buf.len();
        println!("------- TODO parse response! {} bytes available", num_bytes_total);
        match self.state {
            CodecState::ParsingHeaders => {
                let (num_bytes_remaining, mut response) = match parser::response(buf.as_ref()) {
                    IResult::Incomplete(_) => return Ok(None), // not enough data
                    IResult::Error(e) => return Err(Error::new(ErrorKind::Other, e)),
                    IResult::Done(rest, response) => (rest.len(), response)
                };

                println!("num_bytes_remaining = {}", num_bytes_remaining);
                let buf = buf.drain_to(num_bytes_total - num_bytes_remaining);

                if response.status() == 204 {
                    // no content
                    assert!(num_bytes_remaining == 0);
                    Ok(Some(response))
                } else {
                    let length = 13;
                    if length > num_bytes_remaining {
                        self.state = CodecState::ReadingBody(response);
                        Ok(None) // not enough data
                    } else {
                        response.append(buf.as_slice());
                        Ok(Some(response))
                    }
                }
            }

            CodecState::ReadingBody(_) => {
                Ok(None)
            }
        }
    }

    fn encode(&mut self, msg: HttpRequest, buf: &mut Vec<u8>) -> io::Result<()> {
        write!(buf, "{}", msg)?;
        buf.extend_from_slice(&msg.body);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;

    //use std::env;
    use std::io::{Error, ErrorKind};
    use std::thread;
    use std::time::Duration;

    use super::prelude::*;
    use {HttpRequest, HttpCodec, mpsc};

    //#[test]
    fn channel() {
        // Create the event loop that will drive this server
        let string = "http://localhost:3000/post-test".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4]).unwrap()
            .header("Content-Type", "text/plain");

        let mut core = Core::new().unwrap();
        let addr = req.addr().unwrap();
        let handle = core.handle();

        let (mut sender, receiver) = mpsc::channel(1);

        thread::spawn(|| {
            for i in 0 .. 4 {
                let url = "http://localhost:3000/post-test";
                let elements = (0 .. (i + 1)).collect::<Vec<_>>();
                let req = HttpRequest::post(url, elements).unwrap()
                    .header("Content-Type", "text/plain");
                sender = sender.send(req).wait().unwrap();
                thread::sleep(Duration::from_millis(100));
            }
        });


        let _framed = core.run(TcpStream::connect(&addr, &handle).and_then(|connection| {
            let framed = connection.framed(HttpCodec::new());
            receiver.fold(framed, |framed, req| {
                req.send(framed).and_then(|(res, framed)| {
                    println!("channel got response {:?}", res);
                    Ok(framed)
                }).map_err(|_| ())
            }).map_err(|()| Error::new(ErrorKind::Other, "oops"))
        })).unwrap();
    }

    #[test]
    fn two_frames() {
        // Create the event loop that will drive this server
        let string = "http://localhost:3000/post-test".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4, 5, 6]).unwrap()
            .header("Content-Type", "text/plain");

        let mut core = Core::new().unwrap();
        let addr = req.addr().unwrap();
        let handle = core.handle();
        let (res, framed) = core.run(TcpStream::connect(&addr, &handle).and_then(|connection| {
            let framed = connection.framed(HttpCodec::new());
            req.send(framed)
        })).unwrap();
        println!("hello 1 {:?}", res);

        thread::sleep(Duration::from_secs(1));

        let req = HttpRequest::get("http://localhost:3000/").unwrap();
        let (res, _framed) = core.run(req.send(framed)).unwrap();
        println!("hello 2 {:?}", res);
    }
}

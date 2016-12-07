extern crate futures;
extern crate tokio_core;
extern crate url;

use std::borrow::Cow;
use std::fmt;
use std::io::{self, ErrorKind, Write};
use std::net::{SocketAddr, ToSocketAddrs};

use futures::{Future, Poll, Async};

use tokio_core::io::{EasyBuf, Codec};

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

    pub fn addr(&self) -> Result<SocketAddr, io::Error> {
        let mut addrs = self.url.to_socket_addrs()?;
        addrs.next().ok_or(io::Error::new(ErrorKind::UnexpectedEof, "no address"))
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
pub struct HttpResponse;

impl Future for HttpResponse {
    type Item = HttpResponse;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::Ready(HttpResponse))
    }
}

#[derive(Debug)]
pub struct HttpCodec;

impl Codec for HttpCodec {
    type In = HttpResponse;
    type Out = HttpRequest;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, io::Error> {
        let len = buf.len();
        println!("------- TODO parse response! {} bytes available", len);
        if len == 0 {
            Ok(None)
        } else {
            buf.drain_to(len);
            Ok(Some(HttpResponse))
        }
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        println!("encode");
        write!(buf, "{}", msg)?;
        buf.extend_from_slice(&msg.body);
        println!("{:?}", buf);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;

    //use std::env;

    use futures::{Future, Sink, Stream};

    use tokio_core::net::TcpStream;
    use tokio_core::io::Io;
    use tokio_core::reactor::Core;

    use {HttpRequest, HttpCodec};

    #[test]
    fn collect_one() {
        //env::set_var("RUST_LOG", "TRACE");
        //env_logger::init().unwrap();

        // Create the event loop that will drive this server
        let string = "http://localhost:3000/segment/chunks".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4]).unwrap()
            .header("Content-Type", "text/plain")
            .header("Connection", "Close");

        let mut core = Core::new().unwrap();
        let addr = req.addr().unwrap();
        let handle = core.handle();
        let tcp_stream = core.run(TcpStream::connect(&addr, &handle)).unwrap();
        let framed = tcp_stream.framed(HttpCodec);

        // collect a single response since we use Connection: Close
        let res = core.run(framed.send(req).and_then(|framed| framed.collect())).unwrap();
        println!("hello collect {:?}", res);
    }

    #[test]
    fn for_each() {
        println!("TODO send multiple requests in future loop");
    }

    #[test]
    fn it_works() {
        // Create the event loop that will drive this server
        let string = "http://localhost:3000/segment/chunks".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4]).unwrap()
            .header("Content-Type", "text/plain");

        let mut core = Core::new().unwrap();
        let addr = req.addr().unwrap();
        let handle = core.handle();
        let tcp_stream = core.run(TcpStream::connect(&addr, &handle)).unwrap();
        let framed = tcp_stream.framed(HttpCodec);

        let framed = core.run(framed.send(req)).unwrap();
        let (res, framed) = core.run(framed.into_future()).ok().unwrap();
        println!("hello 1 {:?}", res);



        let req = HttpRequest::post(&string, vec![1, 2, 3]).unwrap()
            .header("Content-Type", "text/plain");

        let framed = core.run(framed.send(req)).unwrap();

        let (res, _framed) = core.run(framed.into_future()).ok().unwrap();
        println!("hello 2 {:?}", res);
    }
}

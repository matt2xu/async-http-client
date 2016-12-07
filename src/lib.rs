extern crate futures;
extern crate tokio_core;
extern crate url;

use std::borrow::Cow;
use std::fmt;
use std::io::{self, ErrorKind, Write};
use std::net::{SocketAddr, ToSocketAddrs};

use futures::{Future, Poll, Async, Sink, Stream};

use tokio_core::io::{EasyBuf, Codec, Framed, Io, IoFuture};

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
pub struct HttpResponse;

impl Future for HttpResponse {
    type Item = HttpResponse;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::Ready(HttpResponse))
    }
}

#[derive(Debug)]
pub struct HttpCodec {
    parsed: bool
}

impl HttpCodec {
    pub fn new() -> HttpCodec {
        HttpCodec {parsed: false}
    }
}

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
            self.parsed = true;
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
    use std::thread;
    use std::time::Duration;

    use futures::{Future, Sink, Stream};

    use tokio_core::net::TcpStream;
    use tokio_core::io::Io;
    use tokio_core::reactor::Core;

    use {HttpRequest, HttpCodec};

    /*#[test]
    fn test() {
        use futures::sync::mpsc;

        let (tx1, rx1) = mpsc::channel::<i32>(1);
        let (tx2, rx2) = mpsc::channel::<i32>(1);
        let (tx3, rx3) = mpsc::channel(1);

        thread::spawn(move || {
            tx1.send(1).wait().unwrap()
               .send(2).wait().unwrap();
        });
        thread::spawn(|| {
            tx2.send(3).wait().unwrap()
               .send(4).wait().unwrap();
        });
        thread::spawn(|| {
            tx3.send(rx1).wait().unwrap()
               .send(rx2).wait().unwrap();
        });

        let mut result = rx3.flatten().collect();
        assert_eq!(result.wait(), Ok(vec![1, 2, 3, 4]));
    }*/

    //#[test]
    fn for_each() {
        // Create the event loop that will drive this server
        let string = "http://localhost:3000/segment/chunks".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4]).unwrap()
            .header("Content-Type", "text/plain");

        let mut core = Core::new().unwrap();
        let addr = req.addr().unwrap();
        let handle = core.handle();
        let tcp_stream = core.run(TcpStream::connect(&addr, &handle)).unwrap();
        let framed = tcp_stream.framed(HttpCodec::new());

        // collect a single response since we use Connection: Close
        let (sink, stream) = framed.split();
        let sink = core.run(sink.send(req).and_then(|sink| {
            stream.fold(sink, |sink, res| {
                println!("for each {:?}", res);
                thread::sleep(Duration::from_secs(1));
                let url = "http://localhost:3000/segment/chunks";
                sink.send(HttpRequest::post(url, vec![1, 2, 3]).unwrap()
                    .header("Content-Type", "text/plain"))
            })
        })).unwrap();
    }

    //#[test]
    fn channel() {
        use std::sync::mpsc;

        // Create the event loop that will drive this server
        let string = "http://localhost:3000/segment/chunks".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4]).unwrap()
            .header("Content-Type", "text/plain");

        let mut core = Core::new().unwrap();
        let addr = req.addr().unwrap();
        let handle = core.handle();
        let tcp_stream = core.run(TcpStream::connect(&addr, &handle)).unwrap();
        let framed = tcp_stream.framed(HttpCodec::new());

        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            loop {
                let url = "http://localhost:3000/segment/chunks";
                let req = HttpRequest::post(url, vec![1, 2, 3]).unwrap()
                    .header("Content-Type", "text/plain");
                sender.send(req).unwrap();
                thread::sleep(Duration::from_secs(1));
            }
        });

        // collect a single response since we use Connection: Close
        let req = receiver.recv().unwrap();
        let (sink, stream) = framed.split();
        let framed = core.run(sink.send(req).and_then(|sink| {
            stream.fold(sink, |sink, res| {
                println!("channel {:?}", res);
                let req = receiver.recv().unwrap();
                sink.send(req)
            })
        })).unwrap();
    }

    #[test]
    fn two_frames() {
        // Create the event loop that will drive this server
        let string = "http://localhost:3000/segment/chunks".to_string();
        let req = HttpRequest::post(&string, vec![1, 2, 3, 4]).unwrap()
            .header("Content-Type", "text/plain");

        let mut core = Core::new().unwrap();
        let addr = req.addr().unwrap();
        let handle = core.handle();
        let tcp_stream = core.run(TcpStream::connect(&addr, &handle)).unwrap();
        let framed = tcp_stream.framed(HttpCodec::new());

        let (res, framed) = core.run(req.send(framed)).unwrap();
        println!("hello 1 {:?}", res);

        let req = HttpRequest::post(&string, vec![1, 2, 3]).unwrap()
            .header("Content-Type", "text/plain");

        let (res, _framed) = core.run(req.send(framed)).unwrap();
        println!("hello 2 {:?}", res);
    }
}

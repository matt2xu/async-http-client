// Example is taken from
// https://github.com/matt2xu/async-http-client/issues/1

extern crate async_http_client;
extern crate tokio_tls;
extern crate native_tls;

use std::io::{Error, ErrorKind};

use async_http_client::prelude::*;
use async_http_client::{HttpRequest, HttpCodec};

use self::native_tls::TlsConnector;
use self::tokio_tls::TlsConnectorExt;

fn main() {
    let hostname = "https://www.google.com";
    let req = HttpRequest::get(hostname).unwrap();
    let mut core = Core::new().unwrap();
    let addr = req.addr().unwrap();

    let handle = core.handle();
    let cx = TlsConnector::builder().unwrap().build().unwrap();

    let connection = TcpStream::connect(&addr, &handle);

    let tls_handshake = connection.and_then(|socket| {
        cx.connect_async(hostname, socket).map_err(|e| {
            Error::new(ErrorKind::Other, e)
        })
    });

    let (res, _) = core.run(tls_handshake.and_then(|connection| {
        let framed = connection.framed(HttpCodec::new());
        let result = req.send(framed);
        return result;
    })).unwrap();

    println!("{}", res.unwrap());
}
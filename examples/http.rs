// Example is taken from
// http://matt2xu.github.io/async-http-client

extern crate async_http_client;

use async_http_client::prelude::*;
use async_http_client::{HttpRequest, HttpCodec};

fn main() {
    let req = HttpRequest::get("http://www.google.com").unwrap();
    let mut core = Core::new().unwrap();
    let addr = req.addr().unwrap();
    let handle = core.handle();
    let (res, _) = core.run(TcpStream::connect(&addr, &handle).and_then(|connection| {
        let framed = connection.framed(HttpCodec::new());
        req.send(framed)
    })).unwrap();
    println!("got response {}", res.unwrap());
}

//! Parser module for HTTP response.

use nom::{IResult, ErrorKind, crlf, is_digit, is_space};
use nom::IResult::{Done, Error};

use super::{HttpResponse, Header};
use response::{new_response, new_header};

use std::str;

fn is_token(c: u8) -> bool {
    c > 32 && c < 127 && c != b':'
}

fn parse_code(input: &[u8]) -> IResult<&[u8], u32> {
    if input.iter().all(|&c| is_digit(c)) {
        let sum = input.iter().fold(
            0,
            |sum, &c| sum * 10 + (c as char).to_digit(10).unwrap_or(0),
        );
        Done(&input[3..], sum)
    } else {
        Error(ErrorKind::Digit)
    }
}

struct Status {
    major: u32,
    minor: u32,
    code: u32,
}

impl Status {
    fn version(&self) -> (u32, u32) {
        (self.major, self.minor)
    }
}

named!(status_line<Status>,
    do_parse!(
        tag!("HTTP/") >>
        major: take_while1!(is_digit) >>
        char!('.') >>
        minor: take_while1!(is_digit) >>
        char!(' ') >>
        code: flat_map!(take!(3), parse_code) >>
        char!(' ') >>
        take_until!("\r\n") >>
        crlf >>
        ({
            // this is safe because major and minor only contain digits
            let major = unsafe { str::from_utf8_unchecked(major) }.parse().unwrap_or(0);
            let minor = unsafe { str::from_utf8_unchecked(minor) }.parse().unwrap_or(0);
            Status {
                major: major,
                minor: minor,
                code: code
            }
        })
    )
);


fn trim_right(value: &[u8]) -> &[u8] {
    &value[..value.iter().rposition(|&c| !is_space(c)).map_or(
        0,
        |pos| pos + 1,
    )]
}

named!(header_field<Header>,
    do_parse!(
        name: take_while1!(is_token) >>
        char!(':') >>
        take_while!(is_space) >>
        value: take_until_s!("\r\n") >>
        crlf >>
        ({
            new_header(
                // this is safe because is_token only keeps 32 < c < 127
                unsafe { str::from_utf8_unchecked(name) },
                String::from_utf8_lossy(trim_right(value)).into_owned()
            )
        })
    )
);

named!(pub response<HttpResponse>,
    do_parse!(
        status: status_line >>
        headers: many1!(header_field) >>
        crlf >>
        ({
            new_response(status.version(), status.code, headers)
        })
    )
);

#[cfg(test)]
mod tests {
    use super::response;

    #[test]
    fn test_response() {
        let result = response(
            b"HTTP/1.1 404 Not Found\r\n\
            Host: localhost:3000 \r\n\
            Content-Length: 5\r\n\
            Transfer-Encoding: GZIP , chunked \r\n\
            Connection: keep-alive\r\n\
            $Dumb!:  \t   \r\n\
            \r\n\
            12345",
        );

        // test parsing
        assert!(result.is_done());
        let (body, res) = result.unwrap();
        assert_eq!(body, &b"12345"[..]);

        // status
        assert_eq!(res.status(), 404);

        // is
        assert!(res.is("Connection", "keep-alive"));
        assert!(res.is("connection", "Keep-Alive"));
        assert!(!res.is("Connection", "close"));
        assert!(!res.is("No header", "present"));

        // trim outside, not inside. Header value represented with same case.
        assert!(res["transfer-encoding"].as_ref().map_or(false, |enc| enc == "GZIP , chunked"));

        // has
        assert!(res.has("Transfer-Encoding", "chunked"));
        assert!(res.has("trAnSfeR-enCodIng", "gzip"));
        assert!(!res.has("Transfer-Encoding", "deflate"));
    }
}

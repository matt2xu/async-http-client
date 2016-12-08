
use nom::{IResult, ErrorKind, crlf, is_digit, is_space};
use nom::IResult::{Done, Error};

use std::str;

fn is_token(c: u8) -> bool {
    c > 32 && c < 127 && c != b':'
}

fn parse_code(input: &[u8]) -> IResult<&[u8], u32> {
    if input.iter().all(|&c| is_digit(c)) {
        let sum = input.iter().fold(0, |sum, &c| sum * 10 + (c as char).to_digit(10).unwrap_or(0));
        Done(&input[3..], sum)
    } else {
        Error(ErrorKind::Digit)
    }
}

pub enum StatusKind {
    Informational,
    Successful,
    Redirection,
    ClientError,
    ServerError
}

#[derive(PartialEq, Eq, Debug)]
pub struct Status {
    pub major: u32,
    pub minor: u32,
    pub code: u32
}

named!(pub status_line<Status>,
    do_parse!(
        tag!("HTTP/") >>
        major: take_while1!(is_digit) >>
        char!('.') >>
        minor: take_while1!(is_digit) >>
        char!(' ') >>
        code: flat_map!(take!(3), parse_code) >>
        char!(' ') >>
        reason: take_until!("\r\n") >>
        call!(crlf) >>
        ({
            // this is safe because major and minor only contain digits
            let major = unsafe { str::from_utf8_unchecked(major) }.parse().unwrap_or(0);
            let minor = unsafe { str::from_utf8_unchecked(minor) }.parse().unwrap_or(0);
            println!("reason: {}", str::from_utf8(reason).unwrap_or("invalid reason"));
            Status {
                major: major,
                minor: minor,
                code: code
            }
        })
    )
);

#[derive(PartialEq, Eq, Debug)]
pub struct Header {
    name:  String,
    value: String
}

fn trim_right(value: &[u8]) -> &[u8] {
    &value[.. value.iter().rposition(|&c| !is_space(c)).map_or(0, |pos| pos + 1)]
}

named!(header_field<Header>,
    do_parse!(
        name: take_while1!(is_token) >>
        char!(':') >>
        take_while!(is_space) >>
        value: take_until_s!("\r\n") >>
        crlf >>
        ({
            Header {
                // this is safe because is_token only keeps 32 < c < 127
                name: unsafe { str::from_utf8_unchecked(name) }.to_owned(),
                value: String::from_utf8_lossy(trim_right(value)).into_owned()
            }
        })
    )
);

#[derive(PartialEq, Eq, Debug)]
pub struct Response {
    pub status: Status,
    pub headers: Vec<Header>
}

named!(pub message<Response>,
    do_parse!(
        status: status_line >>
        headers: many1!(header_field) >>
        crlf >>
        ({
            Response {
                status: status,
                headers: headers
            }
        })
    )
);

#[cfg(test)]
mod tests {
    use super::message;
    use super::{Response, Status, Header};

    use nom::IResult::Done;

    #[test]
    fn test_status_line() {
        assert_eq!(message(&b"HTTP/1.1 404 Not Found\r\n\
            Host: localhost:3000 \r\n\
            Content-Length: 5\r\n\
            $Dumb!:  \t   \r\n\
            \r\n\
            12345"[..]),
            Done(&b"12345"[..], Response {
                status: Status {major: 1, minor:1, code: 404},
                headers: vec![
                    Header {name: "Host".to_string(), value: "localhost:3000".to_string()},
                    Header {name: "Content-Length".to_string(), value: "5".to_string()},
                    Header {name: "$Dumb!".to_string(), value: "".to_string()}
                ]
            }));
    }
}

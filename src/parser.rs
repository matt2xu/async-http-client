
use nom::{IResult, ErrorKind, crlf, is_digit};
use nom::IResult::{Done, Error};

use std::str;

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
pub struct StatusLine {
    pub major: u32,
    pub minor: u32,
    pub code: u32
}

named!(pub status_line<StatusLine>,
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
            let major = unsafe { str::from_utf8_unchecked(major) }.parse().unwrap_or(0);
            let minor = unsafe { str::from_utf8_unchecked(minor) }.parse().unwrap_or(0);
            println!("reason: {}", str::from_utf8(reason).unwrap_or("invalid reason"));
            StatusLine {
                major: major,
                minor: minor,
                code: code
            }
        })
    )
);

#[cfg(test)]
mod tests {
    use super::status_line;
    use super::StatusLine;

    use nom::IResult::Done;

    #[test]
    fn test_status_line() {
        assert_eq!(status_line(&b"HTTP/1.1 404 Not Found\r\n"[..]),
            Done(&b""[..], StatusLine {major: 1, minor:1, code: 404}));
    }
}

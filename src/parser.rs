
use nom::{IResult, ErrorKind, crlf, digit};
use nom::IResult::{Incomplete, Done, Error};

use std::str;

fn ret_digit(input: &[u8]) -> IResult<&[u8], u32> {
    let result = digit(input);
    result.map(|slice| (slice[0] as char).to_digit(10).unwrap_or(0))
}

fn code(input: &[u8]) -> IResult<&[u8], u32> {
    match is_a!(input, &b"0123456789"[..]) {
        Error(e)    => Error(e),
        Incomplete(e) => Incomplete(e),
        Done(i, o) => {
            if o.len() != 3 {
                return Error(ErrorKind::Digit);
            }

            Done(i, str::from_utf8(o).unwrap_or("0").parse().unwrap_or(0))
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct StatusLine {
    pub code: u32
}

named!(pub status_line<StatusLine>,
    do_parse!(
        tag!("HTTP/") >>
        d1 : ret_digit >>
        char!('.') >>
        d2 : ret_digit >>
        char!(' ') >>
        code: flat_map!(take!(3), code) >>
        char!(' ') >>
        reason: take_until!("\r\n") >>
        call!(crlf) >>
        ({
            assert!(d1 == 1);
            assert!(d2 == 1);
            println!("reason: {}", str::from_utf8(reason).unwrap_or("invalid reason"));
            StatusLine {
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
            Done(&b""[..], StatusLine {code: 404}));
    }
}

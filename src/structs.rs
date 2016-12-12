//! Definition of structures.

#[derive(PartialEq, Eq, Debug)]
pub struct Header {
    pub name: String,
    pub value: String
}

#[derive(PartialEq, Eq, Debug)]
pub struct HttpResponse {
    version: (u32, u32),
    status: u32,
    headers: Vec<Header>
}

impl HttpResponse {
    pub fn new(version: (u32, u32), status: u32, headers: Vec<Header>) -> HttpResponse {
        HttpResponse {
            version: version,
            status: status,
            headers: headers
        }
    }

    pub fn status(&self) -> u32 {
        self.status
    }

    pub fn is_informational(&self) -> bool {
        self.status >= 100 && self.status < 200
    }

    pub fn is_successful(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn is_redirection(&self) -> bool {
        self.status >= 300 && self.status < 400
    }

    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }

    pub fn append<A: AsRef<[u8]>>(&mut self, _buf: A) {
    }
}

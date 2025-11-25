#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpStatus {
    Ok,
    BadRequest,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    PayloadTooLarge,
    InternalServerError,
}

impl HttpStatus {
    pub fn code(&self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::BadRequest => 400,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::PayloadTooLarge => 413,
            Self::InternalServerError => 500,
        }
    }

    pub fn text(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::BadRequest => "Bad Request",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::PayloadTooLarge => "Payload Too Large",
            Self::InternalServerError => "Internal Server Error",
        }
    }

    pub fn as_response_line(&self) -> String {
        format!("HTTP/1.1 {} {}\r\n", self.code(), self.text())
    }
}

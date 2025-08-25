use std::fmt;

#[derive(Debug, Clone)]
pub enum ErrorCode {
    BadRequest,
    NotFound,
    Unauthorized,
}

pub struct ApiError {
    code: ErrorCode,
    message: Option<String>,
}

impl std::fmt::Debug for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Code: {:?}, Message: {:?}", self.code, self.message)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(msg) => write!(f, "{:?}: {}", self.code, msg),
            None => write!(f, "{:?}", self.code),
        }
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn new(code: ErrorCode, message: String) -> Self {
        ApiError {
            code,
            message: Some(message),
        }
    }
}

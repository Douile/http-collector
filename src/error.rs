use std::error::Error;
use std::fmt;

pub struct SimpleError {
    description: String,
}

impl fmt::Debug for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description)
    }
}

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description)
    }
}

impl Error for SimpleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

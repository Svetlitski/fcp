use std::error;
use std::fmt;
use std::result;

#[derive(Debug)]
pub struct Error(String);

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Ideally the generic bound would be `T: string::ToString`, but that does not compile because
// specialization is not stable and this would thus conflict with the blanket implementation of
// `From`. This is a workaround for this issue, with the unfortunate consequence that `Error`
// cannot implement `error::Error` (otherwise we'd run into the same issue).
impl<T: error::Error> From<T> for Error {
    fn from(other: T) -> Self {
        Error(other.to_string())
    }
}

impl Error {
    pub fn new(message: String) -> Self {
        Error(message)
    }
}

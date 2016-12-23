use std::error::Error;
use std::fmt::{self, Debug};


#[derive(Debug)]
pub struct EdenServerError {
    pub description: String,
}

impl fmt::Display for EdenServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.description, f)
    }
}


impl Error for EdenServerError {
    fn description(&self) -> &str {
        &*self.description
    }
}

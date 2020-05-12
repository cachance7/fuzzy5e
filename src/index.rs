use std::error::Error;
use std::{fmt, fmt::{Display, Formatter}};

#[derive(Debug)]
pub enum IndexError {
    // ConnectionError,
    ProcessingError,
}

impl Display for IndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)?;
        Ok(())
    }
}

impl Error for IndexError {}
pub trait Indexer : Clone {
    fn index<T: Index>(&self, idx: Box<T>) -> Result<(), IndexError>;
    fn index_bulk<T: Index>(&self, idx: Vec<Box<T>>) -> Result<(), IndexError>;
    fn query_ids(&self, col: &str, query: &str) -> Result<Vec<String>, IndexError>;
    fn query(&self, col: &str, query: &str) -> Result<Vec<(String, Vec<u8>)>, IndexError>;
    fn flush_all(&self, col: &str) -> Result<(), IndexError>;
}

pub trait ToBytes {
    fn to_bytes(&self) -> Vec<u8>;
}

pub trait Index: ToBytes {
    fn id(&self) -> String;
    fn mtype(&self) -> String;
    fn tuples(&self) -> Vec<(String, String, String, String)>;
}

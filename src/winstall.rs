#[derive(PartialEq, Debug, Clone)]
pub enum Error {
    InvalidArgument(String, String),
    MissingOperand(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Backup {
    None,
    Numbered,
    Existing,
    Simple,
}
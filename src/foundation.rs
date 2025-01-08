#[derive(PartialEq, Debug)]
pub enum Operation {
    ShowHelp,
    ShowVersion,
    CopyFiles(Vec<std::path::PathBuf>, std::path::PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(PartialEq, Debug)]
pub enum Error {}

pub trait MessageRouter {
    fn out(&mut self, message: Box<dyn std::fmt::Display>);
    fn err(&mut self, message: Box<dyn std::fmt::Display>);
}

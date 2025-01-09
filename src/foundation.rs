#[derive(PartialEq, Debug)]
pub enum Operation {
    ShowHelp,
    ShowVersion,
    CopyFiles(Vec<std::path::PathBuf>, std::path::PathBuf),
}

pub trait MessageRouter {
    fn out(&mut self, message: Box<dyn std::fmt::Display>);
    fn err(&mut self, message: Box<dyn std::fmt::Display>);
}

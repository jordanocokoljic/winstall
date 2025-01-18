#[derive(PartialEq, Debug)]
pub enum Operation {
    ShowHelp,
    ShowVersion,
    CopyFiles {
        preserve_timestamps: bool,
        files: Vec<std::path::PathBuf>,
        directory: std::path::PathBuf,
    },
    CreateDirectories(Vec<std::path::PathBuf>),
}

#[derive(PartialEq, Debug)]
pub enum BackupStrategy {
    None,
    Numbered,
    Simple(String),
    Existing(String),
}

pub trait MessageRouter {
    fn out(&mut self, message: Box<dyn std::fmt::Display>);
    fn err(&mut self, message: Box<dyn std::fmt::Display>);
}

use std::fs::File;
use std::io;
use std::path::Path;

pub enum BackupMethod {
    None,
    Simple(String),
    Numbered,
    Existing(String),
}

pub trait WorkingDirectory {
    fn open_for_read<P: AsRef<Path>>(&self, path: P) -> io::Result<File>;
    fn open_for_write<P: AsRef<Path>>(&self, path: P, method: BackupMethod) -> io::Result<File>;
    fn create_directory<P: AsRef<Path>>(&self, path: P, make_full: bool) -> io::Result<()>;
}

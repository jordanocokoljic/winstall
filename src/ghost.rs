use std::fs;
use std::path::{Path, PathBuf};

pub struct EphemeralPath<'a> {
    path: &'a str,
}

impl<'a> EphemeralPath<'a> {
    pub fn new(path: &'a str) -> Self {
        fs::create_dir(path).unwrap_or_else(|_| panic!("unable to create: {}", path));
        Self { path }
    }

    pub fn join<P: AsRef<Path>>(&self, suffix: P) -> PathBuf {
        Path::new(self.path).join(suffix)
    }
}

impl<'a> Drop for EphemeralPath<'a> {
    fn drop(&mut self) {
        fs::remove_dir_all(self.path).unwrap_or_else(|_| panic!("unable to delete: {}", self.path))
    }
}

#[cfg(test)]
mod tests {
    use crate::ghost::EphemeralPath;
    use std::fs::File;
    use std::path::Path;

    #[test]
    fn test_ephemeral_path() {
        let path = "test_dir";

        {
            let root = EphemeralPath::new(path);
            File::create(root.join("file.txt")).expect("unable to create file");

            assert!(Path::new(path).exists());
            assert!(Path::new(path).join("file.txt").exists());
        }

        assert!(!Path::new(path).exists());
    }
}

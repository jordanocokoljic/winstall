use std::fs;
use std::path::{Path, PathBuf};

pub struct EphemeralPath {
    path: PathBuf,
}

impl EphemeralPath {
    pub fn new(path: &str) -> Self {
        fs::create_dir(path).unwrap_or_else(|_| panic!("unable to create: {}", path));
        Self {
            path: PathBuf::from(path),
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn join<P: AsRef<Path>>(&self, suffix: P) -> PathBuf {
        self.path().join(suffix)
    }
}

impl Drop for EphemeralPath {
    fn drop(&mut self) {
        fs::remove_dir_all(self.path())
            .unwrap_or_else(|_| panic!("unable to delete: {}", self.path().display()))
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

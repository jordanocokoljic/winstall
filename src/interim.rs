use std::path::{Path, PathBuf};
use std::{fs, io};

pub struct Interim {
    real: PathBuf,
}

impl Interim {
    pub fn new<P: AsRef<Path>>(p: P) -> io::Result<Self> {
        fs::create_dir(&p)?;

        Ok(Self {
            real: p.as_ref().to_path_buf(),
        })
    }

    pub fn join<P: AsRef<Path>>(&self, p: P) -> PathBuf {
        self.real.join(p)
    }
}

impl AsRef<Path> for Interim {
    fn as_ref(&self) -> &Path {
        &self.real
    }
}

impl Drop for Interim {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.real)
            .unwrap_or_else(|_| panic!("unable to remove directory: {}", self.real.display()));
    }
}

#[cfg(test)]
mod tests {
    use crate::interim::Interim;
    use std::path::Path;

    #[test]
    fn interim_new_creates_directory() {
        let name = "interim_new_creates_directory";
        let _path = Interim::new(name).expect("unable to create path");
        assert!(Path::new(name).exists());
    }

    #[test]
    fn interim_removes_directory_on_drop() {
        let name = "path_removes_directory";

        {
            let _path = Interim::new(name).expect("unable to create path");
            assert!(Path::new(name).exists());
        }

        assert!(!Path::new(name).exists());
    }

    #[test]
    fn interim_join_joins_paths() {
        let path = Interim::new("path_join_joins_paths").expect("unable to create path");
        let joined = path.join("next");

        assert_eq!(joined, Path::new("path_join_joins_paths").join("next"));
    }
}

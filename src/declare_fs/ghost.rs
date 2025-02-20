use std::path::{Path, PathBuf};
use std::{fs, io};
use std::ops::Deref;

pub struct Ghost {
    real: PathBuf
}

impl Ghost {
    pub fn new<P: AsRef<Path>>(p: P) -> io::Result<Self> {
        fs::create_dir(&p)?;

        Ok(Self {
            real: p.as_ref().to_path_buf(),
        })
    }
}

impl AsRef<Path> for Ghost {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.real
    }
}

impl Deref for Ghost {
    type Target = Path;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.real.deref()
    }
}

impl Drop for Ghost {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.real).unwrap_or_else(|_| {
            println!("failed to automatically remove ghost directory: {}", self.real.display());
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::declare_fs::ghost::Ghost;

    #[test]
    fn ghost_new_creates_directory() {
        let name = "interim_new_creates_directory";
        let _ghost = Ghost::new(name).expect("unable to create path");
        assert!(Path::new(name).exists());
    }

    #[test]
    fn ghost_new_removes_directory_on_drop() {
        let name = "interim_new_removes_directory_on_drop";

        {
            let _ghost = Ghost::new(name).expect("unable to create path");
            assert!(Path::new(name).exists());
        }

        assert!(!Path::new(name).exists());
    }
}
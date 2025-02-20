use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{fs, io};

pub enum Element {
    File(&'static str, &'static str),
    Directory(&'static str, Vec<Element>),
}

pub fn stage<P: AsRef<Path>>(path: P, elements: Vec<Element>) -> io::Result<()> {
    for element in elements {
        match element {
            Element::File(name, content) => File::create(path.as_ref().join(name))
                .and_then(|mut file| file.write_all(content.as_bytes())),
            Element::Directory(name, elements) => fs::create_dir(path.as_ref().join(name))
                .and_then(|_| stage(path.as_ref().join(name), elements)),
        }?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declare_fs::Ghost;
    use std::fs;

    #[test]
    fn stage_creates_elements() {
        let path = Ghost::new("stage_creates_elements").unwrap();

        let declared = vec![
            Element::File("file.txt", "some content"),
            Element::Directory("dir", vec![Element::File("another.txt", "more content")]),
        ];

        let result = stage(&path, declared);

        assert!(result.is_ok());
        assert_eq!(
            fs::read_to_string(path.join("file.txt")).unwrap(),
            "some content"
        );

        assert!(path.join("dir").exists());
        assert_eq!(
            fs::read_to_string(path.join("dir/another.txt")).unwrap(),
            "more content"
        );
    }
}

mod ghost;
mod stage;

pub use ghost::*;
pub use stage::*;

#[macro_export]
macro_rules! ephemeral {
    ($path:expr) => {
        $crate::declare_fs::ghost::Ghost::new($path)
    };

    ($path:expr, $elements:expr) => {
        $crate::declare_fs::ghost::Ghost::new($path)
            .and_then(|path| $crate::declare_fs::stage::stage(&path, $elements).and_then(|_| Ok(path)))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn ephemeral_can_create_ghost() {
        let name = "ephemeral_creates_ghost";

        {
            let result = ephemeral!(name);

            assert!(result.is_ok());
            assert!(Path::new(name).exists());
        }

        assert!(!Path::new(name).exists());
    }

    #[test]
    fn ephemeral_can_create_ghost_and_stage_elements() {
        let name = "vanish_creates_ghost_and_stages_elements";

        {
            let result = ephemeral!(name, vec![Element::File("file.txt", "some content")]);

            assert!(result.is_ok());
            assert!(Path::new(name).exists());
            assert!(Path::new(name).join("file.txt").exists());
        }

        assert!(!Path::new(name).exists());
    }
}

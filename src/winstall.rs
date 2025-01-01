#[derive(PartialEq, Debug, Clone)]
pub enum Error {
    InvalidBackup(String, String),
    MissingOperand(String),
    PhBadWorkingDirectory,
    DirectoryPermissionDenied(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidBackup(arg, source) => {
                write!(
                    f,
                    r#"invalid argument ‘{}’ for ‘{}’
Valid arguments are:
  - ‘none’, ‘off’
  - ‘simple’, ‘never’
  - ‘existing’, ‘nil’
  - ‘numbered’, ‘t’"#,
                    arg, source
                )
            }
            Error::MissingOperand(operand) => write!(f, "missing {} operand", operand),
            Error::PhBadWorkingDirectory => write!(f, "PH: unable to determine working directory"),
            Error::DirectoryPermissionDenied(dir) => {
                write!(f, "cannot create directory ‘{}’: Permission denied", dir)
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Backup {
    None,
    Numbered,
    Existing,
    Simple,
}

#[cfg(test)]
mod tests {
    use crate::winstall::Error::{DirectoryPermissionDenied, InvalidBackup, MissingOperand};

    #[test]
    fn test_error_display_missing_operand() {
        let err = MissingOperand("operand".to_string());
        assert_eq!(format!("{}", err), "missing operand operand");
    }

    #[test]
    fn test_error_display_invalid_argument() {
        let expected = r#"invalid argument ‘arg’ for ‘source’
Valid arguments are:
  - ‘none’, ‘off’
  - ‘simple’, ‘never’
  - ‘existing’, ‘nil’
  - ‘numbered’, ‘t’"#;

        let err = InvalidBackup("arg".to_string(), "source".to_string());
        assert_eq!(format!("{}", err), expected);
    }

    #[test]
    fn test_error_directory_permission_denied() {
        let err = DirectoryPermissionDenied("dir".to_string());
        assert_eq!(format!("{}", err), "cannot create directory ‘dir’: Permission denied")
    }
}

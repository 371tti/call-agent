use std::fmt;
use std::error::Error;

/// アプリケーションで使うエラー型
#[derive(Debug)]
pub enum ClientError {
    /// ファイルが見つからなかった場合など
    NotFound(String),
    /// 入力が不正な場合
    InvalidInput(String),
    /// I/O操作中のエラー
    IoError(std::io::Error),
    IndexOutOfBounds,
    ToolNotFound,
    InvalidEndpoint,
    InvalidPrompt,
    NetworkError,
    InvalidResponse,
    ModelConfigNotSet,
    UnknownError,
}

/// Implements the Display trait for ClientError, providing human-readable error messages
/// for each variant.
///
/// This implementation ensures that all error messages are consistently formatted across
/// the crate. Depending on the specific variant, a descriptive message or the underlying
/// error details (e.g., for I/O errors) are displayed.
///
/// Error Variants:
/// - NotFound: Indicates a missing resource or item. The message provides additional context.
/// - InvalidInput: Denotes that the provided input is not valid. The message explains the issue.
/// - IoError: Wraps a standard I/O error, relaying the system error message.
/// - IndexOutOfBounds: Indicates that an index is outside the allowable bounds.
/// - ToolNotFound: Signals that a required external tool was not found.
/// - InvalidEndpoint: Denotes that a specified endpoint URL or address is invalid.
/// - InvalidPrompt: Indicates that a provided prompt does not meet expected criteria.
/// - NetworkError: Reflects issues with network connectivity or communication.
/// - InvalidResponse: Indicates that the response received does not match the expected format.
/// - UnknownError: A catch-all for errors that do not fit any of the other categories.
///
/// These messages are intended for crate users and are provided in English to support clarity
/// and internationalization.
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::NotFound(ref msg) => write!(f, "NotFound: {}", msg),
            ClientError::InvalidInput(ref msg) => write!(f, "InvalidInput: {}", msg),
            ClientError::IoError(ref err) => write!(f, "IoError: {}", err),
            ClientError::IndexOutOfBounds => write!(f, "Index out of bounds"),
            ClientError::ToolNotFound => write!(f, "Tool not found"),
            ClientError::InvalidEndpoint => write!(f, "Invalid endpoint"),
            ClientError::InvalidPrompt => write!(f, "Invalid prompt"),
            ClientError::NetworkError => write!(f, "Network error"),
            ClientError::InvalidResponse => write!(f, "Invalid response"),
            ClientError::ModelConfigNotSet => write!(f, "Model config not set"),
            ClientError::UnknownError => write!(f, "Unknown error"),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::IoError(ref err) => Some(err),
            _ => None,
        }
    }
}

// std::io::ErrorからAppErrorへの変換を可能にする
impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::IoError(err)
    }
}

impl From<String> for ClientError {
    fn from(err: String) -> Self {
        ClientError::InvalidInput(err)
    }
}
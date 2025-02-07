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
    // 他のエラー型も必要に応じて追加可能
    IndexOutOfBounds,
    ToolNotFound,
    InvalidEndpoint,
    InvalidPrompt,
    NetworkError,
    InvalidResponse,
    UnknownError,
}

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
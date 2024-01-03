use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, AssetError>;

#[derive(Error, Debug)]
pub enum AssetError {
  #[error("file error: {0}")]
  FileError(#[from] std::io::Error),
  #[error("archive error: {0}")]
  ArchiveError(#[from] zip::result::ZipError),
  #[error("failed to process asset raw data: {0}")]
  DataError(#[from] bincode::Error),
  #[error("failed to process asset json: [0]")]
  JsonError(#[from] serde_json::Error),
  #[error("incorrect asset type, expected {0}, got {1}")]
  IncorrectType(&'static str, &'static str),
  #[error("asset version is older than currently supported")]
  OldVersion,
}

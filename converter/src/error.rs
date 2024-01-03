use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, ConverterError>;

#[derive(Error, Debug)]
pub(crate) enum ConverterError {
  #[error("failed to parse arguments: {0}")]
  ArgsError(&'static str),
  #[error("error loading asset: {0}")]
  AssetError(#[from] asset_lib::AssetError),
  #[error("tried to access a resource that doesn't exist!")]
  MissingResource,
  #[error("couldn't parse resource: {0}")]
  ParsingError(&'static str),
}

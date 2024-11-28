use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReportError {
  #[error("Invalid repository URL format")]
  InvalidRepoUrl,

  #[error("Failed to fetch package information: {0}")]
  PackageFetchError(String),

  #[error("Worksheet operation failed: {0}")]
  WorksheetError(String),
}

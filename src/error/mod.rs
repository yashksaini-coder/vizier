//! Error types for Vizier

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VizierError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Cargo metadata error: {0}")]
    CargoMetadata(#[from] cargo_metadata::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, VizierError>;

impl From<syn::Error> for VizierError {
    fn from(e: syn::Error) -> Self {
        VizierError::Parse(e.to_string())
    }
}

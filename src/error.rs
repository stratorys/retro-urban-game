use std::fmt;

#[derive(Debug)]
pub enum GameError {
    AssetLoad { path: String, detail: String },
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AssetLoad { path, detail } => {
                write!(f, "Failed to load asset '{path}': {detail}")
            }
        }
    }
}

impl std::error::Error for GameError {}

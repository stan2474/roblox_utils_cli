use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConversionError { // mesh conversion errors, should probably make this more general later
    #[error("failed to parse obj file: {0}")]
    ObjParse(#[from] tobj::LoadError),

    #[error("failed to find processable mesh data in obj file")]
    NoMeshData,

    #[error("an i/o error occurred: {0}")]
    Io(#[from] io::Error),

    #[error("an unsupported operation was attempted: {0}")]
    Unsupported(String),

    #[error("failed to parse roblox mesh: {0}")]
    RobloxMeshParse(String),
}

pub type Result<T> = std::result::Result<T, ConversionError>;
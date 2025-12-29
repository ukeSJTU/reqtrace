//! Custom error types for ReqTrace

#![allow(dead_code)]

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during requirement parsing
#[derive(Error, Debug)]
pub enum RequirementError {
    #[error("Failed to read requirement file: {path}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid frontmatter in {path}: {message}")]
    FrontmatterError { path: PathBuf, message: String },

    #[error("Missing required field '{field}' in {path}")]
    MissingFieldError { path: PathBuf, field: String },

    #[error("Invalid YAML syntax in {path}")]
    YamlError {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("Failed to parse frontmatter: {0}")]
    ParseError(String),
}

/// Errors that can occur during code scanning
#[derive(Error, Debug)]
pub enum ScanError {
    #[error("Failed to read code file: {path}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Tree-sitter parsing failed for {path}")]
    ParseError { path: PathBuf },

    #[error("Invalid query for language {language}")]
    QueryError {
        language: String,
        #[source]
        source: tree_sitter::QueryError,
    },

    #[error("Unsupported file extension: {extension}")]
    UnsupportedExtension { extension: String },

    #[error("Regex compilation failed: {0}")]
    RegexError(#[from] regex::Error),
}

/// Errors that can occur during file discovery
#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Invalid glob pattern: {pattern}")]
    InvalidGlob {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Directory traversal error: {path}")]
    WalkDirError {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
}

/// Top-level error type for the application
#[derive(Error, Debug)]
pub enum ReqTraceError {
    #[error(transparent)]
    Requirement(#[from] RequirementError),

    #[error(transparent)]
    Scan(#[from] ScanError),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error("No requirement files found")]
    NoRequirements,

    #[error("No code files found")]
    NoCodeFiles,

    #[error("Incomplete coverage: {uncovered} requirements not traced")]
    IncompleteCoverage { uncovered: usize },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// Helper functions for creating errors
impl RequirementError {
    pub fn read_error(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadError { path, source }
    }

    pub fn yaml_error(path: PathBuf, source: serde_yaml::Error) -> Self {
        Self::YamlError { path, source }
    }
}

impl ScanError {
    pub fn read_error(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadError { path, source }
    }

    pub fn query_error(language: String, source: tree_sitter::QueryError) -> Self {
        Self::QueryError { language, source }
    }
}

impl DiscoveryError {
    pub fn invalid_glob(pattern: String, source: globset::Error) -> Self {
        Self::InvalidGlob { pattern, source }
    }

    pub fn walk_dir_error(path: PathBuf, source: walkdir::Error) -> Self {
        Self::WalkDirError { path, source }
    }
}

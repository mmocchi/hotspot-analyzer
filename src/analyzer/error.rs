use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalyzerError {
    #[error("Git error: {0}")]
    GitError(#[from] git2::Error),

    #[error("Invalid repository path")]
    InvalidRepository,

    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    #[error("Analysis error: {0}")]
    AnalysisError(String),
}

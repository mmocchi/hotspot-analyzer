//! エラー型を定義するモジュール
//!
//! このモジュールでは、hotspot分析プロセス中に発生する可能性のある
//! 様々なエラーケースを表現するための列挙型を提供します。

use thiserror::Error;

/// hotspot分析中に発生する可能性のあるエラーを表す列挙型
///
/// # 列挙型の種類
/// - `GitError` - Git操作に関連するエラー
/// - `InvalidRepository` - 無効なGitリポジトリパスが指定された場合のエラー
/// - `InvalidPattern` - 無効なパターンが指定された場合のエラー
/// - `AnalysisError` - コード分析プロセス中の一般的なエラー
#[derive(Error, Debug)]
pub enum AnalyzerError {
    /// Git操作中に発生したエラー
    #[error("Git error: {0}")]
    GitError(#[from] git2::Error),

    /// 指定されたパスが有効なGitリポジトリではない場合のエラー
    #[error("Invalid repository path")]
    InvalidRepository,

    /// パターンマッチングなどで無効なパターンが指定された場合のエラー
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    /// hotspot分析プロセス中に発生した一般的なエラー
    #[error("Analysis error: {0}")]
    AnalysisError(String),

    /// タイムスタンプ関連のエラー
    #[error("Timestamp error: {0}")]
    TimestampError(String),

    /// メトリクス計算時のエラー
    #[error("Metrics calculation error: {0}")]
    MetricsError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        // GitErrorのテスト
        let git_error = git2::Error::from_str("test git error");
        let analyzer_error = AnalyzerError::GitError(git_error);
        assert_eq!(analyzer_error.to_string(), "Git error: test git error");

        // InvalidRepositoryのテスト
        let error = AnalyzerError::InvalidRepository;
        assert_eq!(error.to_string(), "Invalid repository path");

        // InvalidPatternのテスト
        let error = AnalyzerError::InvalidPattern("invalid regex".to_string());
        assert_eq!(error.to_string(), "Invalid pattern: invalid regex");

        // AnalysisErrorのテスト
        let error = AnalyzerError::AnalysisError("analysis failed".to_string());
        assert_eq!(error.to_string(), "Analysis error: analysis failed");

        // TimestampErrorのテスト
        let error = AnalyzerError::TimestampError("invalid time".to_string());
        assert_eq!(error.to_string(), "Timestamp error: invalid time");

        // MetricsErrorのテスト
        let error = AnalyzerError::MetricsError("calculation error".to_string());
        assert_eq!(
            error.to_string(),
            "Metrics calculation error: calculation error"
        );
    }
}

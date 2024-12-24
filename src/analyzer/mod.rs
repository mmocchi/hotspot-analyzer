//! ホットスポット分析の中核となるモジュール
//!
//! このモジュールは、Gitリポジトリの分析に必要な主要なコンポーネントを提供します。
//! 分析プロセスは以下の流れで行われます：
//!
//! 1. 指定された期間内のコミット履歴の取得
//! 2. ファイルごとの変更統計の収集
//! 3. 開発者の貢献度の計算
//! 4. ホットスポットスコアの算出
//!
//! # 主要なコンポーネント
//!
//! - `HotspotAnalyzer`: 分析プロセス全体を制御する主要なクラス
//! - `FileMetrics`: 個々のファイルの分析結果を保持する構造体
//! - `FileStats`: ファイルごとの統計情報を収集する内部構造体

mod error;
mod git;
mod metrics;

pub use error::AnalyzerError;
use git::GitRepository;
pub use metrics::FileMetrics;

use chrono::Utc;
use std::collections::{HashMap, HashSet};

/// ホットスポット分析を実行するメインの構造体
///
/// この構造体は、Gitリポジトリの分析を制御し、
/// 指定された期間内のコミット履歴からホットスポットを特定します。
///
/// # フィールド
///
/// - `repo`: Gitリポジトリへのアクセスを管理するインスタンス
/// - `time_window_days`: 分析対象期間（日数）
pub struct HotspotAnalyzer {
    repo: GitRepository,
    time_window_days: i64,
}

impl HotspotAnalyzer {
    /// 新しいHotspotAnalyzerインスタンスを作成します
    ///
    /// # 引数
    ///
    /// - `path`: 分析対象のGitリポジトリパス
    /// - `time_window_days`: 分析対象期間（日数）
    /// - `include_patterns`: 分析対象とするファイルパターンのリスト
    /// - `exclude_patterns`: 分析から除外するファイルパターンのリスト
    /// - `include_merges`: マージコミットを含めるかどうか
    ///
    /// # エラー
    ///
    /// 以下の場合にエラーを返します：
    /// - 指定されたパスが有効なGitリポジトリでない
    /// - パターンが無効な正規表現として解釈できない
    pub fn new(
        path: impl AsRef<std::path::Path>,
        time_window_days: i64,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        include_merges: bool,
    ) -> Result<Self, AnalyzerError> {
        Ok(Self {
            repo: GitRepository::open(path, include_patterns, exclude_patterns, include_merges)?,
            time_window_days,
        })
    }

    /// リポジトリの分析を実行し、ホットスポットメトリクスを計算します
    ///
    /// # 戻り値
    ///
    /// 分析対象の各ファイルに対する`FileMetrics`のベクターを返します。
    ///
    /// # エラー
    ///
    /// 以下の場合にエラーを返します：
    /// - Gitリポジトリの操作に失敗
    /// - コミット履歴の取得に失敗
    pub fn analyze(&self) -> Result<Vec<FileMetrics>, AnalyzerError> {
        let since = Utc::now() - chrono::Duration::days(self.time_window_days);
        let commits = self.repo.get_commits_since(since)?;

        let mut file_stats: HashMap<String, FileStats> = HashMap::new();

        for commit in commits {
            let author = commit.author.clone();
            for file_path in commit.files {
                let stats = file_stats.entry(file_path).or_default();

                stats.revisions += 1;
                stats.authors.insert(author.clone());
                *stats.author_commits.entry(author.clone()).or_insert(0) += 1;
            }
        }

        Ok(file_stats
            .into_iter()
            .map(|(path, stats)| stats.into_metrics(path))
            .collect())
    }
}

/// ファイルごとの統計情報を収集する内部構造体
///
/// # フィールド
///
/// - `revisions`: ファイルの変更回数
/// - `authors`: ファイルを変更した開発者のセット
/// - `author_commits`: 開発者ごとのコミット回数
#[derive(Default)]
struct FileStats {
    revisions: u32,
    authors: HashSet<String>,
    author_commits: HashMap<String, u32>,
}

impl FileStats {
    /// 収集した統計情報からメトリクスを計算します
    ///
    /// # 引数
    ///
    /// - `path`: 対象ファイルのパス
    ///
    /// # 戻り値
    ///
    /// 計算された`FileMetrics`インスタンスを返します
    fn into_metrics(self, path: String) -> FileMetrics {
        let total_commits: u32 = self.author_commits.values().sum();

        let (main_contributor_percentage, knowledge_distribution) = if total_commits > 0 {
            let max_author_commits = self.author_commits.values().max().unwrap_or(&0);
            let percentage = (*max_author_commits as f64 / total_commits as f64) * 100.0;
            let distribution = 1.0 - (percentage / 100.0);
            (percentage, distribution)
        } else {
            (0.0, 0.0)
        };

        let complexity_factor = (self.authors.len() as f64).sqrt();
        let hotspot_score = self.revisions as f64 * complexity_factor * knowledge_distribution;

        FileMetrics {
            path,
            hotspot_score,
            revisions: self.revisions,
            author_count: self.authors.len() as u32,
            main_contributor_percentage,
            knowledge_distribution,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_file_stats_into_metrics() {
        let mut stats = FileStats {
            revisions: 10,
            authors: HashSet::new(),
            author_commits: HashMap::new(),
        };

        // 開発者の貢献を追加
        stats.authors.insert("dev1".to_string());
        stats.authors.insert("dev2".to_string());
        stats.author_commits.insert("dev1".to_string(), 7);
        stats.author_commits.insert("dev2".to_string(), 3);

        let metrics = stats.into_metrics("test.rs".to_string());

        assert_eq!(metrics.path, "test.rs");
        assert_eq!(metrics.revisions, 10);
        assert_eq!(metrics.author_count, 2);

        // メインの貢献者の割合は70%
        assert!((metrics.main_contributor_percentage - 70.0).abs() < 0.001);

        // 知識分布は0.3 (1.0 - 0.7)
        assert!((metrics.knowledge_distribution - 0.3).abs() < 0.001);

        // ホットスポットスコアの検証
        let expected_score = 10.0 * (2.0_f64).sqrt() * 0.3;
        assert!((metrics.hotspot_score - expected_score).abs() < 0.001);
    }

    #[test]
    fn test_empty_file_stats() {
        let stats = FileStats::default();
        let metrics = stats.into_metrics("empty.rs".to_string());

        assert_eq!(metrics.revisions, 0);
        assert_eq!(metrics.author_count, 0);
        assert_eq!(metrics.main_contributor_percentage, 0.0);
        assert_eq!(metrics.knowledge_distribution, 0.0);
        assert_eq!(metrics.hotspot_score, 0.0);
    }
}

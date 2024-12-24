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
    use git2::{Repository, Signature};
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

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

    // ヘルパー関数も修正
    fn create_test_repo() -> Result<(TempDir, Repository), git2::Error> {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path())?;
        let signature = Signature::now("test", "test@example.com")?;

        // test.rsファイルを作成
        fs::write(
            temp_dir.path().join("test.rs"),
            "fn main() { println!(\"Hello\"); }",
        )
        .unwrap();

        // ファイルをステージングに追加
        {
            let mut index = repo.index()?;
            index.add_path(Path::new("test.rs"))?;
            index.write()?;

            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;

            // 初期コミットを作成（親コミットなし）
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )?;

            // コミットが正しく作成されたか確認
            let head = repo.head()?;

            assert!(head.is_branch());
            let commit = head.peel_to_commit()?;

            assert_eq!(commit.parents().len(), 0);
        }
        Ok((temp_dir, repo))
    }

    #[test]
    fn test_analyzer_initialization() -> Result<(), Box<dyn std::error::Error>> {
        let (temp_dir, _) = create_test_repo()?;

        // 正常な初期化
        let analyzer = HotspotAnalyzer::new(
            temp_dir.path(),
            30,
            vec!["**/*.rs".to_string()],
            vec!["**/target/**".to_string()],
            false,
        );
        assert!(analyzer.is_ok());

        // 無効なパスでの初期化
        let invalid_analyzer = HotspotAnalyzer::new("non_existent_path", 30, vec![], vec![], false);
        assert!(invalid_analyzer.is_err());

        Ok(())
    }

    #[test]
    fn test_analyze_empty_repo() -> Result<(), Box<dyn std::error::Error>> {
        let (temp_dir, _) = create_test_repo()?;

        let analyzer = HotspotAnalyzer::new(
            temp_dir.path(),
            30,
            vec!["**/*.txt".to_string()],
            vec![],
            false,
        )?;

        let result = analyzer.analyze()?;
        // 新規リポジトリのため、結果は空か最小限のはず
        assert!(result.is_empty());

        if let Some(metrics) = result.first() {
            assert_eq!(metrics.revisions, 1); // 初期コミットのみ
            assert_eq!(metrics.author_count, 1); // 単一の作者
            assert_eq!(metrics.main_contributor_percentage, 100.0); // 100%単一作者
        }

        Ok(())
    }

    #[test]
    fn test_analyze_single_commits() -> Result<(), Box<dyn std::error::Error>> {
        let (temp_dir, _) = create_test_repo()?;

        let analyzer =
            HotspotAnalyzer::new(temp_dir.path(), 30, vec!["*.rs".to_string()], vec![], false)?;

        let result = analyzer.analyze()?;
        assert!(result.len() == 1);

        if let Some(metrics) = result.first() {
            assert_eq!(metrics.revisions, 1); // 初期コミットのみ
            assert_eq!(metrics.author_count, 1); // 単一の作者
            assert_eq!(metrics.main_contributor_percentage, 100.0); // 100%単一作者
        }

        Ok(())
    }

    #[test]
    fn test_analyze_with_multiple_commits() -> Result<(), Box<dyn std::error::Error>> {
        let (temp_dir, repo) = create_test_repo()?;
        let signature = Signature::now("test2", "test2@example.com")?;

        // 2人目の開発者による変更を追加
        fs::write(
            temp_dir.path().join("test.rs"),
            "fn main() { println!(\"Hello, World!\"); }",
        )
        .unwrap();

        let mut index = repo.index()?;
        index.add_path(Path::new("test.rs"))?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let parent = repo.head()?.peel_to_commit()?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Second commit",
            &tree,
            &[&parent],
        )?;

        let analyzer =
            HotspotAnalyzer::new(temp_dir.path(), 30, vec!["*.rs".to_string()], vec![], false)?;

        let result = analyzer.analyze()?;
        assert_eq!(result.len(), 1);

        let metrics = &result[0];
        assert_eq!(metrics.revisions, 2); // 2回のコミット
        assert_eq!(metrics.author_count, 2); // 2人の作者
        assert!(metrics.main_contributor_percentage <= 100.0);
        assert!(metrics.main_contributor_percentage >= 50.0);
        assert!(metrics.knowledge_distribution > 0.0);
        assert!(metrics.hotspot_score > 0.0);

        Ok(())
    }

    #[test]
    fn test_file_stats_metrics_calculation() {
        let mut stats = FileStats::default();

        // 複数の開発者のコミットを追加
        stats.revisions = 10;
        stats.authors.insert("dev1".to_string());
        stats.authors.insert("dev2".to_string());
        stats.authors.insert("dev3".to_string());

        stats.author_commits.insert("dev1".to_string(), 5);
        stats.author_commits.insert("dev2".to_string(), 3);
        stats.author_commits.insert("dev3".to_string(), 2);

        let metrics = stats.into_metrics("test_file.rs".to_string());

        assert_eq!(metrics.path, "test_file.rs");
        assert_eq!(metrics.revisions, 10);
        assert_eq!(metrics.author_count, 3);

        // メインコントリビューターの割合は50%
        assert!((metrics.main_contributor_percentage - 50.0).abs() < 0.001);

        // 知識分布は0.5 (1.0 - 0.5)
        assert!((metrics.knowledge_distribution - 0.5).abs() < 0.001);

        // 複雑性係数は sqrt(3)
        let complexity_factor = (3.0_f64).sqrt();
        let expected_score = 10.0 * complexity_factor * 0.5;
        assert!((metrics.hotspot_score - expected_score).abs() < 0.001);
    }

    #[test]
    fn test_analyze_with_exclusions() -> Result<(), Box<dyn std::error::Error>> {
        let (temp_dir, _) = create_test_repo()?;

        // 除外パターンに一致するファイルを追加
        fs::write(
            temp_dir.path().join("test.generated.rs"),
            "// Generated code",
        )
        .unwrap();

        let analyzer = HotspotAnalyzer::new(
            temp_dir.path(),
            30,
            vec!["*.rs".to_string()],
            vec!["**/*.generated.rs".to_string()],
            false,
        )?;

        let result = analyzer.analyze()?;

        assert!(result.len() == 1);

        // 生成されたファイルは除外されているはず
        for metrics in &result {
            assert!(!metrics.path.contains(".generated.rs"));
        }

        Ok(())
    }

    #[test]
    fn test_file_stats_edge_cases() {
        // 空の統計
        let empty_stats = FileStats::default();
        let metrics = empty_stats.into_metrics("empty.rs".to_string());
        assert_eq!(metrics.hotspot_score, 0.0);
        assert_eq!(metrics.knowledge_distribution, 0.0);
        assert_eq!(metrics.main_contributor_percentage, 0.0);

        // 単一の開発者
        let mut single_author_stats = FileStats::default();
        single_author_stats.revisions = 1;
        single_author_stats.authors.insert("dev1".to_string());
        single_author_stats
            .author_commits
            .insert("dev1".to_string(), 1);

        let metrics = single_author_stats.into_metrics("single.rs".to_string());
        assert_eq!(metrics.main_contributor_percentage, 100.0);
        assert_eq!(metrics.knowledge_distribution, 0.0);

        // 同等の貢献度
        let mut equal_stats = FileStats::default();
        equal_stats.revisions = 4;
        equal_stats.authors.insert("dev1".to_string());
        equal_stats.authors.insert("dev2".to_string());
        equal_stats.author_commits.insert("dev1".to_string(), 2);
        equal_stats.author_commits.insert("dev2".to_string(), 2);

        let metrics = equal_stats.into_metrics("equal.rs".to_string());
        assert_eq!(metrics.main_contributor_percentage, 50.0);
        assert_eq!(metrics.knowledge_distribution, 0.5);
    }
}

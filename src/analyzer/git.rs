//! Gitリポジトリとの対話を担当するモジュール
//!
//! このモジュールは、libgit2を使用してGitリポジトリからコミット履歴を取得し、
//! ファイルの変更履歴を追跡するための機能を提供します。

use super::error::AnalyzerError;
use chrono::{DateTime, Utc};
use git2::{Commit, Repository};
use regex::Regex;
use std::path::Path;

/// Gitリポジトリへのアクセスを管理する構造体
///
/// # フィールド
///
/// - `repo`: libgit2のリポジトリハンドル
/// - `include_patterns`: 分析対象とするファイルパターン
/// - `exclude_patterns`: 分析から除外するファイルパターン
/// - `include_merge_commits`: マージコミットを含めるかどうかのフラグ
pub struct GitRepository {
    repo: Repository,
    include_patterns: Vec<Regex>,
    exclude_patterns: Vec<Regex>,
    include_merge_commits: bool,
}

/// コミット情報を保持する構造体
///
/// # フィールド
///
/// - `author`: コミット作成者の名前
/// - `files`: コミットで変更されたファイルのリスト
/// - `timestamp`: コミットのタイムスタンプ（分析時の時間フィルタリングに使用）
#[derive(Debug)]
pub struct CommitInfo {
    pub author: String,
    pub files: Vec<String>,
}

impl GitRepository {
    /// 指定されたパスのGitリポジトリをオープンします
    ///
    /// # 引数
    ///
    /// - `path`: Gitリポジトリのパス
    /// - `include_patterns`: 分析対象とするファイルパターン
    /// - `exclude_patterns`: 分析から除外するファイルパターン
    /// - `include_merge_commits`: マージコミットを含めるかどうか
    ///
    /// # エラー
    ///
    /// 以下の場合にエラーを返します：
    /// - リポジトリのオープンに失敗
    /// - パターンの正規表現への変換に失敗
    pub fn open(
        path: impl AsRef<Path>,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        include_merge_commits: bool,
    ) -> Result<Self, AnalyzerError> {
        let repo = Repository::open(path)?;

        let include_patterns = include_patterns
            .into_iter()
            .map(|p| Regex::new(&glob_to_regex(&p)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AnalyzerError::InvalidPattern(e.to_string()))?;

        let exclude_patterns = exclude_patterns
            .into_iter()
            .map(|p| Regex::new(&glob_to_regex(&p)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AnalyzerError::InvalidPattern(e.to_string()))?;

        Ok(Self {
            repo,
            include_patterns,
            exclude_patterns,
            include_merge_commits,
        })
    }

    /// 指定されたファイルパスが分析対象に含まれるかどうかを判定します
    ///
    /// # 引数
    ///
    /// - `file_path`: 判定対象のファイルパス
    ///
    /// # 戻り値
    ///
    /// ファイルが分析対象に含まれる場合は`true`、それ以外は`false`
    fn should_include_file(&self, file_path: &str) -> bool {
        if self
            .exclude_patterns
            .iter()
            .any(|pattern| pattern.is_match(file_path))
        {
            return false;
        }

        if self.include_patterns.is_empty() {
            return true;
        }

        self.include_patterns
            .iter()
            .any(|pattern| pattern.is_match(file_path))
    }

    /// 指定された日時以降のコミット情報を取得します
    ///
    /// # 引数
    ///
    /// - `since`: この日時以降のコミットを取得
    ///
    /// # 戻り値
    ///
    /// コミット情報のベクターを返します
    ///
    /// # エラー
    ///
    /// 以下の場合にエラーを返します：
    /// - コミット履歴の取得に失敗
    /// - コミット情報の解析に失敗
    pub fn get_commits_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<CommitInfo>, AnalyzerError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            let commit_time =
                DateTime::from_timestamp(commit.time().seconds(), 0).ok_or_else(|| {
                    AnalyzerError::AnalysisError("Invalid commit timestamp".to_string())
                })?;

            // 指定された日時より前のコミットはスキップ
            if commit_time < since {
                continue;
            }

            // マージコミットを除外
            if !self.include_merge_commits && commit.parent_count() > 1 {
                continue;
            }

            let author = commit.author().name().unwrap_or("unknown").to_string();

            let files: Vec<String> = self
                .get_changed_files(&commit)?
                .into_iter()
                .filter(|file_path| self.should_include_file(file_path))
                .collect();

            // 変更されたファイルがある場合はコミット情報を追加
            if !files.is_empty() {
                commits.push(CommitInfo { author, files });
            }
        }

        Ok(commits)
    }

    fn get_changed_files(&self, commit: &Commit) -> Result<Vec<String>, AnalyzerError> {
        let tree = commit.tree()?;
        let parent_tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        let mut files = Vec::new();
        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path() {
                if let Some(path_str) = path.to_str() {
                    files.push(path_str.to_string());
                }
            }
        }

        Ok(files)
    }
}

fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::with_capacity(pattern.len() * 2);
    regex.push('^');

    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                let is_double_star = chars.peek() == Some(&'*');
                if is_double_star {
                    chars.next(); // Skip second '*'
                    regex.push_str(if chars.peek() == Some(&'/') {
                        chars.next();
                        ".*/"
                    } else {
                        ".*"
                    });
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push('.'),
            '.' => regex.push_str("\\."),
            '/' => regex.push('/'),
            c if c.is_alphanumeric() => regex.push(c),
            c => regex.push_str(&regex::escape(&c.to_string())),
        }
    }

    regex.push('$');
    regex
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_glob_to_regex() {
        let test_cases = [
            ("*.py", "^[^/]*\\.py$"),
            ("src/*.rs", "^src/[^/]*\\.rs$"),
            ("**/*.js", "^.*/[^/]*\\.js$"),
            ("src/**/*.ts", "^src/.*/[^/]*\\.ts$"),
            ("doc/*.md", "^doc/[^/]*\\.md$"),
            ("test/**", "^test/.*$"),
            ("**.txt", "^.*\\.txt$"),
        ];

        for (input, expected) in test_cases {
            let result = glob_to_regex(input);
            assert_eq!(
                result, expected,
                "Pattern '{}' should convert to '{}', but got '{}'",
                input, expected, result
            );
        }
    }
    #[test]
    fn test_should_include_file() {
        let repo = Repository::open(".").unwrap();
        let git_repo = GitRepository {
            repo,
            include_patterns: vec![
                Regex::new("^.*\\.rs$").unwrap(),
                Regex::new("^src/.*\\.toml$").unwrap(),
            ],
            exclude_patterns: vec![Regex::new("^target/.*$").unwrap()],
            include_merge_commits: false,
        };

        assert!(git_repo.should_include_file("src/main.rs"));
        assert!(git_repo.should_include_file("src/config.toml"));
        assert!(!git_repo.should_include_file("src/main.py"));
        assert!(!git_repo.should_include_file("target/debug/main.rs"));
    }

    #[test]
    fn test_git_repository_open_invalid_path() {
        let result = GitRepository::open("non_existent_path", vec![], vec![], false);
        assert!(result.is_err());
        match result {
            Err(AnalyzerError::GitError(_)) => (),
            _ => panic!("Expected GitError"),
        }
    }

    #[test]
    fn test_glob_to_regex_special_cases() {
        let test_cases = [
            // 特殊文字を含むパターン
            ("doc/(a|b).md", "^doc/\\(a\\|b\\)\\.md$"),
            // 複数のワイルドカードパターン
            ("**/*.min.*", "^.*/[^/]*\\.min\\.[^/]*$"),
            // ドット付きパターン
            (".gitignore", "^\\.gitignore$"),
            ("*.config.js", "^[^/]*\\.config\\.js$"),
            // 複雑なネストパターン
            (
                "src/**/test/**/*.spec.js",
                "^src/.*/test/.*/[^/]*\\.spec\\.js$",
            ),
        ];

        for (input, expected) in test_cases {
            let result = glob_to_regex(input);
            assert_eq!(
                result, expected,
                "Pattern '{}' should convert to '{}', but got '{}'",
                input, expected, result
            );

            // 生成された正規表現が有効であることを確認
            assert!(
                Regex::new(&result).is_ok(),
                "Generated regex '{}' is invalid",
                result
            );
        }
    }

    #[test]
    fn test_should_include_file_edge_cases() {
        let repo = Repository::open(".").unwrap();
        let git_repo = GitRepository {
            repo,
            include_patterns: vec![
                Regex::new("^.*\\.(rs|toml)$").unwrap(),
                Regex::new("^src/.*$").unwrap(),
            ],
            exclude_patterns: vec![
                Regex::new("^target/.*$").unwrap(),
                Regex::new("^.*\\.generated\\..*$").unwrap(),
            ],
            include_merge_commits: false,
        };

        // 境界ケースのテスト
        assert!(git_repo.should_include_file("src/")); // ディレクトリパス
        assert!(git_repo.should_include_file("src/module/file.rs")); // ネストされたパス
        assert!(git_repo.should_include_file("config.toml")); // ルートのtomlファイル
        assert!(!git_repo.should_include_file("")); // 空のパス
        assert!(!git_repo.should_include_file("target/debug/file.rs")); // 除外ディレクトリ
    }

    #[test]
    fn test_git_repository_with_empty_patterns() {
        let repo = Repository::open(".").unwrap();
        let git_repo = GitRepository {
            repo,
            include_patterns: vec![],
            exclude_patterns: vec![],
            include_merge_commits: false,
        };

        // 空のパターンの場合、全てのファイルが含まれる
        assert!(git_repo.should_include_file("any_file.txt"));
        assert!(git_repo.should_include_file("src/main.rs"));
        assert!(git_repo.should_include_file("deeply/nested/path/file.js"));
    }

    // テンポラリリポジトリを使用したテスト用ヘルパー関数
    fn setup_test_repo() -> Result<(TempDir, Repository), git2::Error> {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path())?;

        // テスト用の初期コミットを作成
        let signature = git2::Signature::now("test", "test@example.com")?;
        {
            let tree_id = {
                let mut index = repo.index()?;
                index.write_tree()?
            };
            let tree = repo.find_tree(tree_id)?;
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )?;
        }

        Ok((temp_dir, repo))
    }

    #[test]
    fn test_git_repository_with_empty_repo() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp_dir, _repo) = setup_test_repo()?;

        // 空のリポジトリでの動作確認
        let git_repo =
            GitRepository::open(_temp_dir.path(), vec!["*.rs".to_string()], vec![], false)?;

        let since = Utc::now() - chrono::Duration::days(1);
        let commits = git_repo.get_commits_since(since)?;

        // 新しいリポジトリなので、コミットは初期コミットのみ
        assert!(commits.is_empty());
        Ok(())
    }
}

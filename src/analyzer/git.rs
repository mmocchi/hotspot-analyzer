use super::error::AnalyzerError;
use chrono::{DateTime, Utc};
use git2::{Commit, Repository};
use regex::Regex;
use std::path::Path;

pub struct GitRepository {
    repo: Repository,
    include_patterns: Vec<Regex>,
    exclude_patterns: Vec<Regex>,
    include_merge_commits: bool,
}

#[derive(Debug)]
pub struct CommitInfo {
    pub author: String,
    pub files: Vec<String>,
}

impl GitRepository {
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

    pub fn get_commits_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<CommitInfo>, AnalyzerError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let since_timestamp = since.timestamp();

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            if commit.time().seconds() < since_timestamp {
                continue;
            }

            if !self.include_merge_commits && commit.parent_count() > 1 {
                continue;
            }

            let author = commit.author().name().unwrap_or("unknown").to_string();

            let files: Vec<String> = self
                .get_changed_files(&commit)?
                .into_iter()
                .filter(|file_path| self.should_include_file(file_path))
                .collect();

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
    let mut regex = String::new();
    regex.push('^');
    
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();  // 2つ目の'*'を消費
                    // **の後のスラッシュをチェック
                    if chars.peek() == Some(&'/') {
                        chars.next();  // '/'を消費
                        regex.push_str(".*/");  // ディレクトリをまたぐマッチング
                    } else {
                        regex.push_str(".*");  // スラッシュがない場合は単純に.*
                    }
                } else {
                    regex.push_str("[^/]*");  // 単一の*は現在のディレクトリ内のみマッチ
                }
            },
            '?' => regex.push('.'),
            '.' => regex.push_str("\\."),
            '/' => regex.push('/'),
            c if c.is_alphanumeric() => regex.push(c),
            _ => regex.push_str(&regex::escape(&c.to_string())),
        }
    }
    
    regex.push('$');
    regex
}

#[cfg(test)]
mod tests {
    use super::*;

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
                result, 
                expected, 
                "Pattern '{}' should convert to '{}', but got '{}'", 
                input, 
                expected,
                result
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
}

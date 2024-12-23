mod git;
mod metrics;
mod error;

use git::GitRepository;
pub use metrics::FileMetrics;
pub use error::AnalyzerError;

use std::collections::{HashMap, HashSet};
use chrono::Utc;

pub struct HotspotAnalyzer {
    repo: GitRepository,
    time_window_days: i64,
}

impl HotspotAnalyzer {
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

    pub fn analyze(&self) -> Result<Vec<FileMetrics>, AnalyzerError> {
        let since = Utc::now() - chrono::Duration::days(self.time_window_days);
        let commits = self.repo.get_commits_since(since)?;

        let mut file_stats: HashMap<String, FileStats> = HashMap::new();
        
        for commit in commits {
            let author = commit.author.clone();
            for file_path in commit.files {
                let stats = file_stats
                    .entry(file_path)
                    .or_insert_with(FileStats::default);
                
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

#[derive(Default)]
struct FileStats {
    revisions: u32,
    authors: HashSet<String>,
    author_commits: HashMap<String, u32>,
}

impl FileStats {
    fn into_metrics(self, path: String) -> FileMetrics {
        let total_commits: u32 = self.author_commits.values().sum();
        let max_author_commits = self.author_commits.values().max().cloned().unwrap_or(0);
        
        let main_contributor_percentage = if total_commits > 0 {
            (max_author_commits as f64 / total_commits as f64) * 100.0
        } else {
            0.0
        };
        
        let knowledge_distribution = 1.0 - (main_contributor_percentage / 100.0);
        let hotspot_score = self.revisions as f64 * 
                           self.authors.len() as f64 * 
                           knowledge_distribution;

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
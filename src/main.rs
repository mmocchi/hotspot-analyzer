use anyhow::Context;
use clap::Parser;
use hotspot_analyzer::HotspotAnalyzer;
use std::path::PathBuf;

/// デフォルトのインクルードパターン
const DEFAULT_INCLUDE_PATTERNS: &[&str] = &[
    "**/*.rs",    // Rustファイル
    "**/*.go",    // Goファイル
    "**/*.js",    // JavaScriptファイル
    "**/*.ts",    // TypeScriptファイル
    "**/*.py",    // Pythonファイル
    "**/*.java",  // Javaファイル
    "**/*.cpp",   // C++ファイル
    "**/*.hpp",   // C++ヘッダー
    "**/*.c",     // Cファイル
    "**/*.h",     // Cヘッダー
];

/// デフォルトの除外パターン
const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &[
    "**/target/**/*",      // Rustのビルドディレクトリ
    "**/node_modules/**/*", // Node.jsの依存関係
    "**/dist/**/*",        // ビルド成果物
    "**/build/**/*",       // ビルドディレクトリ
    "**/.git/**/*",        // Gitディレクトリ
    "**/vendor/**/*",      // 依存関係
    "**/*.min.*",          // minifyされたファイル
    "**/test/**/*",        // テストディレクトリ
    "**/tests/**/*",       // テストディレクトリ
    "test/**/*",        // テストディレクトリ
    "tests/**/*",       // テストディレクトリ
];

#[derive(Parser)]
#[command(
    version,
    about = "Analyzes Git repositories to identify code hotspots",
    long_about = None
)]
struct Cli {
    /// Path to Git repository
    #[arg(short, long)]
    repo: PathBuf,

    /// Time window in days
    #[arg(short = 'w', long = "time-window", default_value_t = 365)]
    time_window: i64,

    /// Output format (json or csv)
    #[arg(short, long, default_value = "json")]
    format: String,

    /// Number of top hotspots to show
    #[arg(short = 'n', long, default_value_t = 10)]
    top: usize,

    /// Include only files matching these patterns (glob format, e.g., "*.rs", "src/**/*.py")
    /// If not specified, default includes common source code files
    #[arg(short = 'i', long = "include")]
    include_patterns: Option<Vec<String>>,

    /// Exclude files matching these patterns
    /// If not specified, excludes common build and dependency directories
    #[arg(short = 'e', long = "exclude")]
    exclude_patterns: Option<Vec<String>>,

    /// Use no default include patterns
    #[arg(long)]
    no_default_includes: bool,

    /// Use no default exclude patterns
    #[arg(long)]
    no_default_excludes: bool,

    /// Include merge commits in the analysis
    #[arg(long, default_value_t = false)]
    include_merges: bool,
}

impl Cli {
    fn get_include_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        
        if !self.no_default_includes {
            patterns.extend(DEFAULT_INCLUDE_PATTERNS.iter().map(|s| s.to_string()));
        }
        
        if let Some(ref user_patterns) = self.include_patterns {
            patterns.extend(user_patterns.clone());
        }
        
        patterns
    }

    fn get_exclude_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        
        if !self.no_default_excludes {
            patterns.extend(DEFAULT_EXCLUDE_PATTERNS.iter().map(|s| s.to_string()));
        }
        
        if let Some(ref user_patterns) = self.exclude_patterns {
            patterns.extend(user_patterns.clone());
        }
        
        patterns
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let analyzer = HotspotAnalyzer::new(
        &cli.repo,
        cli.time_window,
        cli.get_include_patterns(),
        cli.get_exclude_patterns(),
        cli.include_merges,
    ).context("Failed to initialize analyzer")?;

    let mut hotspots = analyzer.analyze()
        .context("Failed to analyze repository")?;
    
    hotspots.sort_by(|a, b| b.hotspot_score.partial_cmp(&a.hotspot_score).unwrap());
    let top_hotspots: Vec<_> = hotspots.into_iter().take(cli.top).collect();

    match cli.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&top_hotspots)
                .context("Failed to serialize to JSON")?);
        }
        "csv" => {
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            for metric in top_hotspots {
                wtr.serialize(metric)
                    .context("Failed to write CSV record")?;
            }
            wtr.flush().context("Failed to flush CSV writer")?;
        }
        _ => anyhow::bail!("Unsupported output format: {}", cli.format),
    }

    Ok(())
}
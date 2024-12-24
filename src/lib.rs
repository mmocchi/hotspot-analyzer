//! Gitリポジトリのホットスポット分析ツール
//!
//! このクレートは、Gitリポジトリのコミット履歴を分析し、
//! 頻繁に変更され、多くの開発者が関与しているコードの領域（ホットスポット）を
//! 特定するための機能を提供します。
//!
//! # 主な機能
//!
//! - コミット履歴の分析
//! - ファイルごとの変更頻度の追跡
//! - 開発者の貢献度の計算
//! - ホットスポットスコアの算出
//!
//! # 使用例
//!
//! ```no_run
//! use hotspot_analyzer::HotspotAnalyzer;
//!
//! let analyzer = HotspotAnalyzer::new(
//!     "path/to/repo",
//!     365,
//!     vec!["**/*.rs".to_string()],
//!     vec!["**/target/**".to_string()],
//!     false
//! ).unwrap();
//!
//! let metrics = analyzer.analyze().unwrap();
//! ```

pub mod analyzer;
pub use analyzer::HotspotAnalyzer;

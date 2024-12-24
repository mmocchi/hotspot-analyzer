//! メトリクス計算と結果の表現を担当するモジュール
//!
//! このモジュールは、ホットスポット分析の結果を表現するためのデータ構造と、
//! 分析結果のシリアライズに関する機能を提供します。

use serde::{Deserialize, Serialize};

/// ファイルごとの分析メトリクスを保持する構造体
///
/// # フィールド
///
/// - `path`: 分析対象ファイルのパス
/// - `hotspot_score`: 計算されたホットスポットスコア
/// - `revisions`: ファイルの変更回数
/// - `author_count`: ファイルに貢献した開発者の数
/// - `main_contributor_percentage`: 最も貢献度の高い開発者の貢献割合（%）
/// - `knowledge_distribution`: 知識分布スコア（0-1）
#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetrics {
    pub path: String,
    #[serde(serialize_with = "round_to_3", deserialize_with = "deserialize_f64")]
    pub hotspot_score: f64,
    pub revisions: u32,
    pub author_count: u32,
    #[serde(serialize_with = "round_to_3", deserialize_with = "deserialize_f64")]
    pub main_contributor_percentage: f64,
    #[serde(serialize_with = "round_to_3", deserialize_with = "deserialize_f64")]
    pub knowledge_distribution: f64,
}

/// 浮動小数点数を3桁に丸める補助関数
///
/// # 引数
///
/// - `value`: 丸める浮動小数点数
/// - `serializer`: serdeシリアライザ
fn round_to_3<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64((*value * 1000.0).round() / 1000.0)
}

/// f64値をデシリアライズする補助関数
fn deserialize_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    f64::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_metrics_serialization() {
        let metrics = FileMetrics {
            path: "src/main.rs".to_string(),
            hotspot_score: 12.3456,
            revisions: 42,
            author_count: 5,
            main_contributor_percentage: 45.6789,
            knowledge_distribution: 0.54321,
        };

        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: FileMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(metrics.path, deserialized.path);
        assert_eq!(metrics.revisions, deserialized.revisions);
        assert_eq!(metrics.author_count, deserialized.author_count);

        // 丸められた値の検証
        assert!((deserialized.hotspot_score - 12.346).abs() < 0.001);
        assert!((deserialized.main_contributor_percentage - 45.679).abs() < 0.001);
        assert!((deserialized.knowledge_distribution - 0.543).abs() < 0.001);
    }

    #[test]
    fn test_round_to_3() {
        #[derive(Serialize)]
        struct TestStruct {
            #[serde(serialize_with = "round_to_3")]
            value: f64,
        }

        let test_cases = vec![
            (1.23456, 1.235),
            (0.12345, 0.123),
            (1.0, 1.0),
            (1.2, 1.2),
            (1.234, 1.234),
        ];

        for (input, expected) in test_cases {
            let test_struct = TestStruct { value: input };
            let json = serde_json::to_value(test_struct).unwrap();
            assert!((json["value"].as_f64().unwrap() - expected).abs() < 0.0001);
        }
    }
}

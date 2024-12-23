#!/bin/bash

# デフォルトの除外パターン
EXCLUDE_PATTERNS=(
    ".git"
    "target"
    "node_modules"
    ".DS_Store"
    "*.pyc"
    "__pycache__"
    "project_structure.md"
)

# 使用方法を表示する関数
show_usage() {
    echo "Usage: $0 <project_path>"
    exit 1
}

# ファイルの言語識別子を取得する関数
get_language() {
    local file="$1"
    case "${file##*.}" in
        rs)     echo "rust" ;;
        py)     echo "python" ;;
        js)     echo "javascript" ;;
        ts)     echo "typescript" ;;
        html)   echo "html" ;;
        css)    echo "css" ;;
        sh)     echo "bash" ;;
        toml)   echo "toml" ;;
        md)     echo "markdown" ;;
        json)   echo "json" ;;
        yml|yaml) echo "yaml" ;;
        *)      echo "" ;;
    esac
}

# 除外パターンのチェック
should_exclude() {
    local path="$1"
    for pattern in "${EXCLUDE_PATTERNS[@]}"; do
        if [[ "$path" == *"$pattern"* ]]; then
            return 0
        fi
    done
    return 1
}

# メイン処理
main() {
    local output_file="project_structure.md"
    local project_path=""

    # コマンドライン引数の解析
    while [[ $# -gt 0 ]]; do
        case $1 in
            *)
                if [[ -z "$project_path" ]]; then
                    project_path="$1"
                else
                    echo "Error: Multiple project paths specified"
                    show_usage
                fi
                shift
                ;;
        esac
    done

    if [[ -z "$project_path" ]]; then
        echo "Error: No project path specified"
        show_usage
    fi

    # project_path がディレクトリでない場合はエラー
    if [[ ! -d "$project_path" ]]; then
        echo "Error: $project_path is not a directory"
        show_usage
    fi

    # 出力ファイルの初期化
    echo "# プロジェクト構造" > "$output_file"
    echo "" >> "$output_file"
    echo "## ディレクトリ構造" >> "$output_file"
    echo "" >> "$output_file"
    echo "\`\`\`" >> "$output_file"

    # ディレクトリ構造の出力
    (cd "$project_path" && find . -not -path '*/\.*' -not -path '*/target/*' -not -path '*/node_modules/*' | sort) >> "$output_file"
    
    echo "\`\`\`" >> "$output_file"
    echo "" >> "$output_file"
    echo "## ファイル内容" >> "$output_file"
    echo "" >> "$output_file"

    # 各ファイルの内容を出力
    while IFS= read -r -d '' file; do
        if should_exclude "$file"; then
            continue
        fi

        if [[ -f "$file" ]]; then
            relative_path="${file#$project_path/}"
            echo "### $relative_path" >> "$output_file"
            echo "" >> "$output_file"

            # ファイルの言語識別子を取得
            lang=$(get_language "$file")
            if [[ -n "$lang" ]]; then
                echo "\`\`\`$lang" >> "$output_file"
            else
                echo "\`\`\`" >> "$output_file"
            fi

            cat "$file" >> "$output_file"
            echo "\`\`\`" >> "$output_file"
            echo "" >> "$output_file"
        fi
    done < <(find "$project_path" -type f -print0)

    echo "プロジェクト構造を $output_file に出力しました。"
}

main "$@"
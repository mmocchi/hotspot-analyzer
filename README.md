# README.md
# Hotspot Analyzer

A Git repository analyzer that identifies code hotspots.

## Installation

Using Homebrew:
```bash
brew tap mmocchi/hotspot-analyzer
brew install hotspot-analyzer
```


## Usage

Basic usage:
```bash
Usage: hotspot-analyzer [OPTIONS] --repo <REPO>

Options:
  -r, --repo <REPO>                 Path to Git repository
  -w, --time-window <TIME_WINDOW>   Time window in days [default: 365]
  -f, --format <FORMAT>             Output format (json or csv) [default: json]
  -n, --top <TOP>                   Number of top hotspots to show [default: 10]
  -i, --include <INCLUDE_PATTERNS>  Include only files matching these patterns (glob format, e.g., "*.rs", "src/**/*.py") If not specified, default includes common source code files
  -e, --exclude <EXCLUDE_PATTERNS>  Exclude files matching these patterns If not specified, excludes common build and dependency directories
      --no-default-includes         Use no default include patterns
      --no-default-excludes         Use no default exclude patterns
      --include-merges              Include merge commits in the analysis
  -h, --help                        Print help
  -V, --version                     Print version
```

## Examples

### デフォルトのパターンを使用
```bash
hotspot-analyzer -r /path/to/repo
```

### デフォルトに加えて追加のパターンを指定
```bash
hotspot-analyzer -r /path/to/repo -i "**/*.sql" -e "**/migrations/*"
```

### デフォルトを無効化して独自のパターンのみを使用
```bash
hotspot-analyzer -r /path/to/repo --no-default-includes -i "src/**/*.rs"
```

### デフォルトの除外を無効化
```bash
hotspot-analyzer -r /path/to/repo --no-default-excludes
```

### 全てのデフォルトを無効化して完全にカスタム設定
```bash
hotspot-analyzer -r /path/to/repo --no-default-includes --no-default-excludes -i "src/**/*.rs" -e "src/generated/*"
```


## License

MIT License

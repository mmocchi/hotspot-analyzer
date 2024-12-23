# README.md
# Hotspot Analyzer

A Git repository analyzer that identifies code hotspots.

## Installation

Using Homebrew:
```bash
brew tap mmocchi/hotspot
brew install hotspot-analyzer
```

Using Cargo:
```bash
cargo install hotspot-analyzer
```

Or download pre-built binaries from the [releases page](https://github.com/mmocchi/hotspot-analyzer/releases).

## Usage

Basic usage:
```bash
hotspot -r /path/to/repo
```

Options:
- `-r, --repo`: Path to Git repository
- `-t, --time-window`: Analysis time window in days (default: 365)
- `-f, --format`: Output format (json or csv, default: json)
- `-n, --top`: Number of top hotspots to show (default: 10)

## Examples

### デフォルトのパターンを使用
```bash
cargo run -- -r ./repo
```

### デフォルトに加えて追加のパターンを指定
```bash
cargo run -- -r ./repo -i "**/*.sql" -e "**/migrations/*"
```

### デフォルトを無効化して独自のパターンのみを使用
```bash
cargo run -- -r ./repo --no-default-includes -i "src/**/*.rs"
```

### デフォルトの除外を無効化
```bash
cargo run -- -r ./repo --no-default-excludes
```

### 全てのデフォルトを無効化して完全にカスタム設定
```bash
cargo run -- -r ./repo --no-default-includes --no-default-excludes -i "src/**/*.rs" -e "src/generated/*"
```


## License

MIT License

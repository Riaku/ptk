<p align="center">
  <img src="https://avatars.githubusercontent.com/u/258253854?v=4" alt="PTK - Rust Token Killer" width="500">
</p>

<p align="center">
  <strong>エージェントが読む bash 出力を最大 90% 削減する高性能 CLI プロキシ</strong>
</p>

<p align="center">
  <a href="https://github.com/rtk-ai/ptk/actions"><img src="https://github.com/rtk-ai/ptk/workflows/Security%20Check/badge.svg" alt="CI"></a>
  <a href="https://github.com/rtk-ai/ptk/releases"><img src="https://img.shields.io/github/v/release/rtk-ai/ptk" alt="Release"></a>
  <a href="#ライセンス"><img src="https://img.shields.io/badge/license-Apache--2.0%20%2B%20BUSL--1.1-blue.svg" alt="License: Apache-2.0 and BUSL-1.1"></a>
  <a href="https://discord.gg/RySmvNF5kF"><img src="https://img.shields.io/discord/1478373640461488159?label=Discord&logo=discord" alt="Discord"></a>
</p>

<p align="center">
  <a href="https://www.rtk-ai.app">ウェブサイト</a> &bull;
  <a href="#インストール">インストール</a> &bull;
  <a href="docs/TROUBLESHOOTING.md">トラブルシューティング</a> &bull;
  <a href="docs/contributing/ARCHITECTURE.md">アーキテクチャ</a> &bull;
  <a href="https://discord.gg/RySmvNF5kF">Discord</a>
</p>

<p align="center">
  <a href="README.md">English</a> &bull;
  <a href="README_fr.md">Francais</a> &bull;
  <a href="README_zh.md">中文</a> &bull;
  <a href="README_ja.md">日本語</a> &bull;
  <a href="README_ko.md">한국어</a> &bull;
  <a href="README_es.md">Espanol</a>
</p>

---

ptk はコマンド出力を LLM コンテキストに届く前にフィルタリング・圧縮します。単一の Rust バイナリ、依存関係ゼロ、オーバーヘッド 10ms 未満。

## PTK がすること

PTK はシェルコマンドを横取りし、エージェントが読む前にその出力を圧縮します。

| 操作 | PTK が出力に対して行うこと |
|------|----------------------------|
| `ls` / `tree` | 1 エントリ 1 行ではなく、ファイル数付きのツリー形式 |
| `cat` / `read` | スマートなファイル読み取り：本文全体ではなくシグネチャと構造 |
| `grep` / `rg` | 長い行を切り詰め、マッチをファイル単位でグループ化 |
| `git status` | コンパクトな stat 形式、状態ごとにグループ化 |
| `git diff` | コンテキストを削減、ヘッダーを除去 |
| `git log` | ハッシュ、作者、件名のみ |
| `git add/commit/push` | 進捗出力全体の代わりに確認行 1 行 |
| `cargo test` / `npm test` | 失敗のみ、成功したテストは件数に集約 |
| `ruff check` | ルールとファイルごとにグループ化 |
| `pytest` | 失敗のみ、トレースバックを短縮 |
| `go test` | NDJSON をパースし、失敗のみ |
| `docker ps` | 必須フィールドのみ |

## 節約の仕組み

PTK はエージェントが読む **bash 出力を最大 90% 削減**します。これが PTK の測定対象であり、請求額が 90% 減ることと同じではありません。

bash 出力は、あなたのプロンプト、システムプロンプト、会話履歴と並ぶ**入力トークンの構成要素のひとつ**にすぎません。そして入力トークン自体も、出力トークンを含む**請求額の一部**でしかありません。削減効果は各段階で薄まります。

PTK が報告するトークン数は `バイト数 / 4` で見積もられています。PTK はトークナイザーを同梱していないため、**割合は信頼できますが、トークンの絶対値はあくまで概算**です。

> 詳しい解説：[PTK の節約の仕組み](docs/guide/resources/savings-explained.md)

## インストール

### Homebrew（推奨）

```bash
brew install ptk
```

### winget（Windows）

Windowsで最も簡単なインストール方法 — コマンド1つ、PATHの設定不要：

```powershell
winget install rtk-ai.ptk
```

### クイックインストール（Linux/macOS）

```bash
curl -fsSL https://raw.githubusercontent.com/rtk-ai/ptk/refs/heads/master/install.sh | sh
```

### Cargo

```bash
cargo install --git https://github.com/rtk-ai/ptk ptk
```

### Apache-2.0 エディション（オープンソースのみ、商用利用可）

BUSL-1.1 圧縮エンジンを含めずにビルドした同じ `ptk` バイナリです。フック書き換えシステムとすべてのコマンドフィルタを Apache-2.0 のみで提供します。

```bash
curl -fsSL https://raw.githubusercontent.com/rtk-ai/ptk/refs/heads/master/install.sh | PTK_EDITION=oss sh
brew install rtk-ai/tap/ptk-oss
cargo install --git https://github.com/rtk-ai/ptk ptk --no-default-features
```

### 確認

```bash
ptk --version   # "ptk 0.27.x" と表示されるはず
ptk gain        # トークン節約統計が表示されるはず
```

## クイックスタート

```bash
# 1. 対応する AI ツール用のフックをインストール
ptk init --global               # Claude Code（デフォルト）
ptk init --agent trae           # Trae（プロジェクト）
ptk init --global --agent trae  # Trae（グローバル）

# 2. 対応する AI ツールを再起動してテスト
git status  # 自動的に ptk git status に書き換え
```

## 仕組み

```
  ptk なし：                                       ptk あり：

  Claude  --git status-->  shell  -->  git          Claude  --git status-->  PTK  -->  git
    ^                                   |             ^                      |          |
    |        ~2,000 tokens（生出力）     |             |   ~200 tokens        | フィルタ |
    +-----------------------------------+             +------- （圧縮済）----+----------+
```

4つの戦略：

1. **スマートフィルタリング** - ノイズを除去（コメント、空白、ボイラープレート）
2. **グルーピング** - 類似項目を集約（ディレクトリ別ファイル、タイプ別エラー）
3. **トランケーション** - 関連コンテキストを保持、冗長性をカット
4. **重複排除** - 繰り返しログ行をカウント付きで統合

## コマンド

> 以下の割合は **bash 出力バイト数の削減率**であり、PTK の `バイト数 / 4` 推定器で測定しています。[節約の仕組み](#節約の仕組み)を参照してください。

### ファイル
```bash
ptk ls .                        # 最適化されたディレクトリツリー
ptk read file.rs                # スマートファイル読み取り
ptk find "*.rs" .               # コンパクトな検索結果
ptk grep "pattern" .            # ファイル別グループ化検索
```

### Git
```bash
ptk git status                  # コンパクトなステータス
ptk git log -n 10               # 1行コミット
ptk git diff                    # 圧縮された diff
ptk git push                    # -> "ok main"
```

### テスト
```bash
ptk jest                        # Jest コンパクト
ptk vitest                      # Vitest コンパクト
ptk pytest                      # Python テスト（-90%）
ptk go test                     # Go テスト（-90%）
ptk test <cmd> [args...]        # 失敗のみ表示（-90%）、argv を直接実行
```

### ビルド & リント
```bash
ptk lint                        # ESLint ルール別グループ化
ptk tsc                         # TypeScript エラーグループ化
ptk cargo build                 # Cargo ビルド（-80%）
ptk ruff check                  # Python リント（-80%）
```

### 分析
```bash
ptk gain                        # 節約統計
ptk gain --graph                # ASCII グラフ（30日間）
ptk discover                    # 見逃した節約機会を発見
```

## ドキュメント

- **[TROUBLESHOOTING.md](docs/guide/resources/troubleshooting.md)** - よくある問題の解決
- **[INSTALL.md](INSTALL.md)** - 詳細インストールガイド
- **[ARCHITECTURE.md](docs/contributing/ARCHITECTURE.md)** - 技術アーキテクチャ

## コントリビュート

コントリビューション歓迎！[GitHub](https://github.com/rtk-ai/ptk) で issue または PR を作成してください。

[Discord](https://discord.gg/RySmvNF5kF) コミュニティに参加。

## ライセンス

Apache 2.0 ライセンス - 詳細は [LICENSE](LICENSE) を参照。

> `engine/` and `proxy/` are licensed BUSL-1.1 — see their LICENSE files.

## 免責事項

詳細は [DISCLAIMER.md](DISCLAIMER.md) を参照。

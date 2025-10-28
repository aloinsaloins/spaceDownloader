# Space Downloader

日本語の後に英語版があります。  
English version follows the Japanese section.

---

## 日本語 (Japanese)

Space Downloader は、X（旧 Twitter）のスペース配信をデスクトップから保存するための Rust 製 GUI アプリケーションです。  
`yt-dlp` と `ffmpeg` を活用して音声を抽出し、キュー管理や進捗表示を備えた快適なダウンロード体験を提供します。

### 特長
- Rust + iced 0.13 で実装したクロスプラットフォーム GUI（Windows / macOS / Linux 対応）
- `yt-dlp` を非同期で実行しつつ、進捗率・速度・ETA をリアルタイム表示
- ダウンロードキューと並列実行（1〜3 件まで）およびキャンセル操作に対応
- SQLite（`history/history.db`）へ履歴を書き込み、将来的な履歴 UI に備えたデータ保持
- Fluent による多言語化（現在は `en-US` / `ja-JP` を同梱）
- 設定ファイルによる保存先ディレクトリや音声フォーマット、ログ出力のカスタマイズ

### リポジトリ構成
```
spaceDownloader/
├─ space-downloader-core/   # yt-dlp 実行・設定・履歴管理を担う Rust ライブラリ
├─ space-downloader-gui/    # iced を用いたデスクトップ GUI
├─ docs/                    # 仕様書などのドキュメント
├─ SpaceDownloader.Core/    # 旧 .NET 実装（参照のみ）
└─ SpaceDownloader.UI/      # 旧 .NET UI（参照のみ）
```

> `SpaceDownloader.Core` / `SpaceDownloader.UI` は初期の .NET 試作版です。現行の Rust 実装とは別系統なので、最新機能は `space-downloader-*` 配下を参照してください。

### 必要環境
- Rust 1.78 以上（Edition 2021）
- `yt-dlp`（PATH に配置、もしくは設定でフルパス指定）
- `ffmpeg`（音声変換に使用）
- OS: Windows 10 以降 / macOS 13 Ventura 以降 / Linux x86_64

### ダウンロード

#### バイナリリリース（推奨）
[GitHub Releases](https://github.com/aloinsaloins/spaceDownloader/releases) から最新版をダウンロードできます:

- **Windows (x64)**: `space-downloader-windows-x64.zip`
- **macOS (Apple Silicon - M1/M2/M3/M4)**: `space-downloader-macos-arm64.zip`

各ZIPファイルには、実行ファイルと必要な依存関係（yt-dlp、ffmpeg）が含まれています。追加のインストールは不要です。

> **注**: Intel Mac向けのビルドは提供していません。Apple Siliconバイナリは、Rosetta 2経由でIntel Macでも動作します。

#### macOS での実行方法
バイナリはAppleによって署名・公証されているため、通常は警告なく実行できます。

もし問題が発生した場合、ターミナルから:
```sh
xattr -d com.apple.quarantine space-downloader-gui
./space-downloader-gui
```

### 開発者向けセットアップ
```sh
git clone https://github.com/aloinsaloins/spaceDownloader.git
cd spaceDownloader

# 依存コマンドが PATH に入っているか確認
yt-dlp --version
ffmpeg -version

# GUI を開発モードで起動
cargo run --package space-downloader-gui
```

リリースビルドを作成する場合:

```sh
cargo build --release --package space-downloader-gui
```

生成された実行ファイルは `target/release/space-downloader-gui`（または各 OS の拡張子付き）に配置されます。

### 使い方
1. アプリを起動し、X スペース配信の URL を入力します。
2. `Download` ボタンを押すとジョブがキューに追加され、進捗バーとログが表示されます。
3. 必要に応じて `Cancel` ボタンでジョブを取り消せます。
4. ダウンロード完了後はファイルパスやエラーメッセージがログ欄に表示されます。

> 現在の GUI はシンプルなキュー表示にフォーカスしています。履歴 UI や詳細設定画面は今後のロードマップ項目です。

### 設定ファイル
初回起動時に設定ファイルが自動生成されます。

- 既定パス: `config/space_downloader.toml`（プラットフォームごとの設定ディレクトリ配下）

```toml
[general]
output_dir = "./"
language = "en-US"          # ja-JP に変更可
theme = "system"            # light / dark / system

[download]
format = "m4a"              # m4a / mp3 / opus
max_retries = 3
timeout_sec = 0             # 0 は無制限
concurrency = 1             # 1〜3 にクランプ

[advanced]
yt_dlp_path = "yt-dlp"
cookie_file = ""
extra_args = []
save_logs = true

[logging]
enabled = true
level = "info"              # error / warn / info / debug
```

設定を変更した後はアプリを再起動してください。`cookie_file` にブラウザからエクスポートしたクッキーを指定すると、認証が必要なスペースにも対応できます。

### 保存されるデータ
- 設定: `config/space_downloader.toml`
- 履歴 DB: `history/history.db`（SQLite / WAL モード）
- ログ: `logs/YYYYMMDD.log`（`advanced.save_logs = true` の場合）

これらはワークスペース内に配置され、Git 追跡から除外されています。

### 開発・テスト
```sh
# フォーマット
cargo fmt

# Lint（警告をエラー扱い）
cargo clippy -- -D warnings

# テスト
cargo test --workspace

# core のみ E2E テスト（yt-dlp が必要）
cargo test --package space-downloader-core -- --ignored
```

詳細な仕様は `docs/space_downloader_spec.md` を参照してください。

### トラブルシューティング
- `yt-dlp が見つかりません`: パッケージマネージャや `pip install yt-dlp` でインストールし、PATH に追加するか `advanced.yt_dlp_path` にフルパスを指定します。
- `ffmpeg が見つかりません`: 各 OS の公式配布やパッケージマネージャから導入し、PATH を設定してください。
- 403 / 401 エラー: 認証が必要なスペースではクッキーが必要な場合があります。ブラウザから抽出したクッキーを `cookie_file` に設定してください。
- 進捗が更新されない: `yt-dlp` のバージョンにより出力形式が異なる可能性があります。最新版への更新を検討してください。

### GitHubでのリリース方法

このプロジェクトはGitHub Actionsを使用して、Windows、macOS（Apple Silicon & Intel）用のバイナリを自動ビルドし、リリースとして配布できます。

#### リリース手順

1. **バージョンタグを作成**:
   ```sh
   git tag v1.0.4
   git push origin v1.0.4
   ```

2. **GitHub Actionsが自動実行**:
   - タグがプッシュされると、`.github/workflows/release.yml`が自動的に実行されます
   - Windows、macOS（Apple Silicon）用のバイナリがビルドされます
   - `yt-dlp`と`ffmpeg`が自動的にダウンロードされ、バンドルされます
   - macOSバイナリはAppleによって署名・公証されます
   - すべてのファイルがZIPアーカイブにパッケージ化されます

3. **リリースの確認**:
   - GitHub上でリリースページに自動的に公開されます
   - ユーザーは依存関係をインストールする必要なく、ZIPをダウンロードして解凍するだけで使用できます

#### 含まれるファイル

各ZIPファイルには以下が含まれます：
- Space Downloader実行ファイル
- yt-dlp（最新版）
- ffmpeg（最新版）
- ffprobe（最新版）

**追加の依存関係は不要です！** すべての必要なツールがバンドルされています。

#### macOS版の署名と公証

macOS版は自動的にコード署名と公証が行われます。ユーザーはセキュリティ警告なしでアプリを実行できます。

署名の設定方法については、[SIGNING.md](SIGNING.md)を参照してください。

### ロードマップ
- 設定画面と履歴ビューの GUI 実装
- アプリ起動時の依存関係チェック UI
- URL ドラッグ＆ドロップ対応
- 完了通知（OS ネイティブ通知）
- 再試行やリトライポリシーの強化
- インストーラ／パッケージングスクリプト

### ライセンス
本リポジトリにはまだライセンスが設定されていません。公開・配布前にライセンスを明示してください。

### コントリビュート
Issue / Pull Request を歓迎します。大きめの変更を計画している場合は、事前に仕様書（`docs/space_downloader_spec.md`）を確認しつつ、Issue で相談いただけるとスムーズです。

---

## English

Space Downloader is a Rust-based desktop GUI application designed to archive Twitter/X Space broadcasts.  
It wraps `yt-dlp` and `ffmpeg` to extract audio while providing queue management, live progress reporting, and cancellation support.

### Highlights
- Cross-platform GUI built with Rust + iced 0.13 (Windows / macOS / Linux)
- Runs `yt-dlp` asynchronously and surfaces progress, speed, and ETA in real time
- Queueing and parallel execution (clamped to 1–3 jobs) with cancel controls
- Persists job history in SQLite (`history/history.db`) for future history screens
- Fluent-based localization (ships with `en-US` and `ja-JP`)
- Configurable output directory, audio format, and logging via TOML settings

### Repository Layout
```
spaceDownloader/
├─ space-downloader-core/   # Rust library for yt-dlp orchestration, settings, history
├─ space-downloader-gui/    # Desktop GUI built with iced
├─ docs/                    # Additional documentation and specifications
├─ SpaceDownloader.Core/    # Legacy .NET prototype (read-only)
└─ SpaceDownloader.UI/      # Legacy .NET UI (read-only)
```

> `SpaceDownloader.Core` and `SpaceDownloader.UI` are early .NET prototypes. The active Rust implementation lives under the `space-downloader-*` directories.

### Requirements
- OS: Windows 10+, macOS 13 Ventura+, or Linux x86_64
- For development: Rust 1.78 or newer (Edition 2021)

### Download

#### Binary Releases (Recommended)
Download the latest version from [GitHub Releases](https://github.com/aloinsaloins/spaceDownloader/releases):

- **Windows (x64)**: `space-downloader-windows-x64.zip`
- **macOS (Apple Silicon - M1/M2/M3/M4)**: `space-downloader-macos-arm64.zip`

Each ZIP file includes the executable and all required dependencies (yt-dlp, ffmpeg). No additional installation needed.

> **Note**: Intel Mac builds are not provided. Apple Silicon binaries work on Intel Macs via Rosetta 2.

#### macOS Security Notice
Binaries are code-signed and notarized by Apple, so they should run without security warnings.

If you encounter issues, from Terminal:
```sh
xattr -d com.apple.quarantine space-downloader-gui
./space-downloader-gui
```

### Developer Setup
```sh
git clone https://github.com/aloinsaloins/spaceDownloader.git
cd spaceDownloader

# Ensure external tools are available
yt-dlp --version
ffmpeg -version

# Launch the GUI in debug mode
cargo run --package space-downloader-gui
```

To build a release binary:
```sh
cargo build --release --package space-downloader-gui
```

The compiled artifact lands under `target/release/space-downloader-gui` (with the appropriate platform extension).

### Usage
1. Start the app and paste a Twitter/X Space URL.
2. Press the `Download` button to enqueue the job and monitor progress/logs.
3. Use `Cancel` to stop an in-flight job when needed.
4. When a job finishes, the file path or error details appear in the log area.

> The current GUI focuses on queue management. History browsing and advanced settings screens are planned roadmap items.

### Configuration
The app creates a configuration file on first launch.

- Default path: `config/space_downloader.toml` (inside the platform-specific config directory)

```toml
[general]
output_dir = "./"
language = "en-US"          # change to ja-JP if desired
theme = "system"            # light / dark / system

[download]
format = "m4a"              # m4a / mp3 / opus
max_retries = 3
timeout_sec = 0             # 0 = unlimited
concurrency = 1             # clamped between 1 and 3

[advanced]
yt_dlp_path = "yt-dlp"
cookie_file = ""
extra_args = []
save_logs = true

[logging]
enabled = true
level = "info"              # error / warn / info / debug
```

Restart the app after changing the file. Providing an exported browser cookie file via `cookie_file` enables access to authenticated spaces.

### Stored Data
- Config: `config/space_downloader.toml`
- History DB: `history/history.db` (SQLite in WAL mode)
- Logs: `logs/YYYYMMDD.log` (when `advanced.save_logs = true`)

All artifacts live within the workspace and are ignored by Git.

### Development & Tests
```sh
# Formatting
cargo fmt

# Linting (treat warnings as errors)
cargo clippy -- -D warnings

# Tests
cargo test --workspace

# Core E2E tests (requires yt-dlp)
cargo test --package space-downloader-core -- --ignored
```

For a deeper dive into requirements and flows, check `docs/space_downloader_spec.md`.

### Troubleshooting
- `yt-dlp not found`: Install via your package manager or `pip install yt-dlp`, then update PATH or set `advanced.yt_dlp_path`.
- `ffmpeg not found`: Install from official builds or your package manager and ensure PATH includes it.
- 403 / 401 errors: Some spaces require authentication. Export cookies from your browser and point `cookie_file` to them.
- Progress not updating: Certain `yt-dlp` versions format output differently—upgrade to the latest release if parsing fails.

### Releasing on GitHub

This project uses GitHub Actions to automatically build binaries for Windows, macOS (Apple Silicon & Intel), and distribute them as releases.

#### Release Steps

1. **Create a version tag**:
   ```sh
   git tag v1.0.4
   git push origin v1.0.4
   ```

2. **GitHub Actions runs automatically**:
   - When the tag is pushed, `.github/workflows/release.yml` automatically executes
   - Builds binaries for Windows and macOS (Apple Silicon)
   - Automatically downloads and bundles `yt-dlp` and `ffmpeg`
   - Code-signs and notarizes macOS binaries with Apple
   - Packages everything into ZIP archives

3. **Check the release**:
   - The release is automatically published on GitHub's release page
   - Users can download and extract the ZIP without needing to install any dependencies

#### What's Included

Each ZIP file contains:
- Space Downloader executable
- yt-dlp (latest version)
- ffmpeg (latest version)
- ffprobe (latest version)

**No additional dependencies required!** All necessary tools are bundled.

#### Code Signing and Notarization (macOS)

macOS binaries are automatically code-signed and notarized. Users can run the app without security warnings.

For signing setup instructions, see [SIGNING.md](SIGNING.md).

### Roadmap
- GUI for settings and history browsing
- Dependency checks during startup
- Drag-and-drop URL support
- Native OS notifications
- Improved retry policies
- Installer / packaging scripts

### License
No license has been chosen yet. Please add one before distributing the project.

### Contributing
Issues and pull requests are welcome. For larger changes, review the spec in `docs/space_downloader_spec.md` and discuss via an issue first to align on scope.


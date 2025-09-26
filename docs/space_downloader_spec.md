# Xスペースダウンローダー GUI 仕様書

## 1. 背景と目的
- X (旧Twitter) のスペース配信はアーカイブが限定的であり、非技術ユーザーでも扱えるダウンロード手段が求められている。
- Rust ベースのダウンロードロジックを活かしつつ、視覚的な操作で `yt-dlp` を呼び出せる GUI アプリケーションを提供する。
- 既存の `SpaceDownloader.Core` (Rust ライブラリ) を中核として再利用し、今後の他プラットフォーム展開にも備える。

## 2. プロジェクトスコープ
### 含む
- クロスプラットフォーム GUI アプリケーション `space-downloader-gui` (Rust + iced) の実装。
- URL 入力・ダウンロード進行表示・完了通知を備えたメイン画面の提供。
- ダウンロード履歴と設定管理 (保存先/形式/リトライなど) の実装。
- `yt-dlp` 実行および成果物/メタデータの保存。
- GUI から依存関係チェック (`yt-dlp`, `ffmpeg`) を実行する機能。
### 含まない
- CLI アプリケーションの提供。
- Cookie 自動取得などのブラウザ連携機能。
- `yt-dlp` バイナリの同梱・配布。
- モバイル (iOS/Android) 対応。

## 3. 想定ユーザー
- X スペースを GUI で簡単に保存したい一般ユーザー。
- `SpaceDownloader.UI` など既存 UI から Rust コア機能を呼び出したい開発者。

## 4. 前提・依存関係
- Rust 1.78 以上 (Rust Edition 2021)。
- `space-downloader-core` クレート (ライブラリ) と `space-downloader-gui` クレート (バイナリ) のワークスペース構成。
- GUI フレームワーク: `iced 0.13` (Tokio 互換の async executor 利用)。
- `yt-dlp` がローカルにインストール済みで PATH に登録されていること。
- 音声変換のため `ffmpeg` が導入済みであること。
- 対応 OS: Windows 10+, macOS 13+, Linux x86_64。

## 5. 主要ユースケース
1. ユーザーが URL を入力して「ダウンロード」を押下し、進捗バーとステータスメッセージを確認しながら音声ファイルを取得する。
2. 出力ディレクトリや音声フォーマットを設定画面で変更し、次回以降に反映される。
3. ダウンロード履歴一覧から対象をダブルクリックしてエクスプローラー/ファインダーで開く。
4. 依存関係チェックを実行し、`yt-dlp`/`ffmpeg` が不足していればガイド付きのモーダルダイアログを表示する。
5. エラー発生時に原因・推奨アクションを表示し、再試行ボタンでリトライする。

## 6. 機能要件
### メイン画面
- URL 入力欄、貼り付け補助ボタン、ダウンロード開始ボタンを配置。
- 進捗表示: 進捗率 (%)、取得済みサイズ、ダウンロード速度、残り時間を表示。
- ステータスログパネル: 最新 50 行をスクロール表示。詳細ログファイルに全出力を保存 (設定で無効化可)。
- 実行中はキャンセルボタンを表示し、中断時に部分ファイルを削除するか確認。

### ダウンロード管理
- ダウンロードジョブはキューイングし、同時実行数は 1 (設定で最大 3 まで拡張可能)。
- ジョブの状態: `Queued`, `Running`, `Succeeded`, `Failed`, `Canceled`。
- 履歴パネルで各ジョブのタイトル、配信者、終了日時、保存先パスを一覧表示。
- 履歴アイテムから「再ダウンロード」「フォルダを開く」を実行可能。

### 設定
- タブ構成: 一般 / ダウンロード / 上級者向け。
  - 一般: 出力ディレクトリ、言語 (日本語/英語)、テーマ (ライト/ダーク/自動)。
  - ダウンロード: 音声フォーマット (`m4a`, `mp3`, `opus`)、リトライ回数、タイムアウト秒数、最大同時実行数。
  - 上級者向け: `yt-dlp` パス、Cookie ファイル指定、追加オプション引数、ログ保存有無。
- 設定は `config/space_downloader.toml` に保存。GUI から `デフォルトに戻す` を提供。

### 依存関係チェック
- アプリ起動時とユーザー操作時に `yt-dlp --version` と `ffmpeg -version` を非同期で実行。
- 未検出の場合、公式ドキュメントへのリンクと PATH 設定手順を表示。

### エラー通知
- 共通エラータイプをダイアログで表示し、「詳細表示」でスタックトレース/コマンド出力を展開。
- リトライ可能なエラー (`RateLimited`, `Timeout`) では待機秒数を提案。

### 国際化
- UI テキストは `en-US` と `ja-JP` を `fluent` で管理。初期値は OS 言語、設定で切り替え。

### 拡張項目 (任意)
- ドラッグ＆ドロップで URL 入力欄にセット。
- 複数 URL を改行区切りで貼り付けた場合、自動的にキュー登録。
- ダウンロード完了時の通知トースト/OS ネイティブ通知。

## 7. 非機能要件
- 初回起動からメイン画面表示まで 3 秒以内 (SSD 想定)。
- ダウンロード中も UI 応答性を維持 (60 fps 目標、GC ブロッキングなし)。
- メモリ使用量: 平常時 200MB 未満。
- ログファイルは 10MB でローテーション、最大 5 世代保持。
- 例外はすべて `anyhow::Error` で捕捉し、ユーザー向けには簡潔な日本語メッセージを表示。

## 8. UI 仕様
### 画面構成
- ヘッダー: アプリ名、ステータスアイコン、設定ボタン。
- コンテンツ 2 カラム:
  - 左: URL 入力、進捗カード、ステータスログ。
  - 右: 履歴リスト、依存関係ステータス、ショートカットリンク。
- フッター: `yt-dlp` バージョン表示、チェック結果アイコン、著作権注意文。

### 状態遷移
- `Idle` → `Validating` (URL 検証) → `Queued` → `Running` → `Succeeded/Failed/Canceled`。
- URL 未入力・検証中はダウンロードボタンを非活性。
- `Failed` 状態ではエラー詳細を展開して再試行ボタンを表示。

### テーマ
- ライト/ダークのカラーパレットをデザインガイドに添付。
- ハイコントラストモードを OS 設定から検知し強調色を調整。

## 9. アーキテクチャ
```
workspace/
  Cargo.toml
  crates/
    space-downloader-core/
      src/
        lib.rs
        config/
        yt_dlp/
        downloader/
        metadata/
        logging.rs
    space-downloader-gui/
      src/
        main.rs
        app.rs
        screens/
          main.rs
          settings.rs
          history.rs
        components/
          progress_card.rs
          job_list.rs
        commands/
          download.rs
          dependency_check.rs
        localization/
```

- `space-downloader-core`: CLI 非依存のビジネスロジックを保持。`DownloaderService` が `yt_dlp::CommandBuilder` と `metadata::Parser` を利用。
- `space-downloader-gui`: `iced::Application` 実装。`tokio` を組み込んだ `Executor` で非同期タスクを起動。
- メッセージ駆動: GUI から `Command` (iced) を発行し、バックグラウンドで `core` の API を実行して `Message::DownloadProgress` 等に変換。
- DI: `AppContext` 構造体が設定、ログハンドラ、タスクスケジューラへのハンドルを保持。

## 10. yt-dlp 連携
- 実行コマンド例:
  `yt-dlp --extract-audio --audio-format {format} --audio-quality 0 --write-info-json --no-playlist --output "{output}" "{url}"`
- 非同期実行で stdout/stderr を逐次読み込み、`ProgressEvent` にマッピング。
- 正常終了後、`<base>.info.json` を読み込んでタイトル・開始時刻などを履歴に登録。
- 終了コード非ゼロの場合は `DownloadError::CommandFailed` を返し、stderr を UI に提示。

## 11. 設定/データ永続化
- 設定ファイル: `config/space_downloader.toml`
  ```
  [general]
  output_dir = "~/Downloads/space-dl"
  language = "ja-JP"
  theme = "system"

  [download]
  format = "m4a"
  overwrite = "skip"
  max_retries = 3
  timeout_sec = 0
  concurrency = 1

  [advanced]
  yt_dlp_path = "yt-dlp"
  cookie_file = ""
  extra_args = []
  save_logs = true
  ```
- 履歴: `history/history.db` (SQLite) にジョブ情報を保存。テーブル `downloads` は `id`, `url`, `title`, `uploader`, `status`, `started_at`, `ended_at`, `file_path`, `error_code`, `error_message` を保持。
- ログ: `logs/YYYYMMDD.log` に `tracing` で出力。GUI から「ログフォルダを開く」を提供。

## 12. エラーハンドリング方針
- エラー分類:
  - `DependencyMissing` (`yt-dlp`, `ffmpeg`) – ガイド付きモーダル。
  - `InvalidUrl` – 入力欄にバリデーションメッセージを表示。
  - `AuthenticationRequired` – Cookie 指定を促す。
  - `RateLimited` – 次回試行までの待機時間を表示。
  - `DownloadTimeout` – タイムアウト値の調整を提案。
- 未分類は「予期しないエラー」としてエラーログへのリンクを表示。

## 13. ログ・トレース
- `tracing` + `tracing-subscriber` を利用し、GUI イベント (`ButtonPressed`, `JobStateChanged`) とコアログ (`yt_dlp::CommandExecuted`) をタグ付け。
- ログレベルは設定画面で `error`, `warn`, `info`, `debug` を切り替え。
- 個人情報に該当する Cookie/トークンはマスクして出力。

## 14. テスト計画
- 単体テスト (`space-downloader-core`): コマンドライン生成、設定マージ、メタデータパーサ、履歴リポジトリ。
- Unit/UI テスト (`space-downloader-gui`): `iced` の `widget::canvas` などを用いた状態遷移テスト、設定保存のモック。
- 統合テスト: `cargo test --package space-downloader-core -- --ignored` でモック `yt-dlp` を用いた end-to-end を実行。
- 手動テスト: 代表的 OS (Win/macOS/Linux) で URL 入力・キャンセル・エラーを確認。依存関係不足ケースを網羅。
- CI: GitHub Actions で `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, `cargo deny`。Windows/macOS/Linux 3 matrix。

## 15. セキュリティ/法的考慮
- 初回起動時に X の利用規約遵守と著作権の注意事項をダイアログで表示し、同意後に利用可能。
- Cookie/トークンファイルパスは設定ファイルに平文保存せず、ユーザー操作時に選択して渡すのみ。
- ログには URL とユーザー識別情報を記録しない (ハッシュ化)。
- 自動アップデートは提供せず、未署名バイナリ配布を想定。

## 16. マイルストーン
1. 仕様確定・UI モック作成 (本ドキュメント + Figma ワイヤー) – 1日
2. `space-downloader-core` の API 整理/切り出し – 1日
3. GUI プロジェクト雛形 (`iced` / 設定保存) – 2日
4. ダウンロードフロー実装 (進捗表示・キャンセル) – 3日
5. 履歴/設定画面・依存チェック – 2日
6. エラー処理・国際化・ロギング – 2日
7. テスト/ドキュメント整備/リリース準備 – 1日

## 17. 今後の拡張候補
- タスクトレイ常駐モードとドラッグ＆ドロップキュー登録。
- `SpaceDownloader.UI` (C# 等) との IPC 連携用 gRPC レイヤーの提供。
- 複数アカウント/認証ヘッダー管理機能。
- 完了後の自動アップロード (S3/Google Drive) 連携。
- ウィザード形式による一括ダウンロードテンプレート。

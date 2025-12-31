# Security Camera Viewer (MJPEG)

Spresense セキュリティカメラから MJPEG ストリームを受信・保存する PC 側アプリケーション。

## ✅ 最新更新 (2025-12-31)

### Phase 3.0: VGA GUI ビューア + パイプライン最適化 🚀

**Option A パイプライン実装完了**:
- JPEG デコードをキャプチャスレッドに移動（GUI スレッドから分離）
- シリアル読み込みとデコード時間の詳細測定
- テクスチャアップロード時間のトラッキング
- 期待性能: 16-20 fps → **25-30 fps** (VGA 640×480)

### Phase 2.0 完了 (2025-12-22)

H.264 プロトコルから MJPEG プロトコルへの完全移行が完了しました。

**主な変更点**:
- **プロトコル**: H.264 NAL Units → MJPEG パケット
- **ヘッダー**: 22 bytes → 14 bytes (SYNC + SEQ + SIZE + CRC16)
- **CRC**: CRC-16-IBM-SDLC → CRC-16-CCITT
- **出力**: .h264 ファイル → .mjpeg ストリームまたは個別 JPEG ファイル
- **JPEG形式**: JFIF形式とベアJPEG形式の両方に対応

## 📋 必要要件

- Rust 1.70+
- Spresense デバイス (VID=0x054C, PID=0x0BC2)
- USB CDC-ACM 接続

## 🚀 ビルド方法

### GUIアプリケーション

```bash
cd /home/ken/Rust_ws/security_camera_viewer

# GUI版をビルド
cargo build --release --features gui --bin security_camera_gui

# 実行
./run_gui.sh
```

### CLIアプリケーション

```bash
# CLI版をビルド
cargo build --release --bin security_camera_viewer
```

## 💻 使用方法

### GUIアプリケーション

リアルタイムでカメラ映像を表示:

```bash
# GUIビューアを起動
./run_gui.sh

# または直接実行
./target/release/security_camera_gui
```

**機能**:
- 📹 リアルタイムMJPEGストリーム表示
- 📊 FPS・フレーム数・エラー統計
- ⏱ **詳細性能メトリクス**: デコード・シリアル読み込み・テクスチャアップロード時間
- ▶️ Start/Stopコントロール
- 🔍 自動検出またはポート指定
- ⚙️ 設定パネル
- 🚀 **Option A パイプライン**: JPEG デコードとテクスチャアップロードの並列処理

**✅ Windows クロスコンパイル対応 (Phase 3.0)**:
- WSL2 から Windows ネイティブ .exe をビルド可能
- MinGW-w64 クロスコンパイラを使用
- WSL2 の OpenGL 制限を回避
- 詳細: [WINDOWS_BUILD.md](WINDOWS_BUILD.md)

**⚠️ WSL2での制限**:
- WSL2環境ではOpenGL/GLXサポートが不完全なため、GUI動作が不安定な場合があります
- **推奨**: Windows クロスコンパイル版を使用（上記参照）
- または下記の「WSL2向け簡易ビューア」をご利用ください

### WSL2向け簡易ビューア (推奨)

WSL2で動作する軽量なライブビューア:

```bash
# 必要なら画像ビューアをインストール
sudo apt-get install feh

# ライブビューア起動
./view_live.sh
```

**仕組み**: 個別JPEGファイルをキャプチャしながら、`feh`で自動更新表示します。

### CLIアプリケーション

コマンドラインで録画:

```bash
# 自動検出モード (推奨)
./target/release/security_camera_viewer

# シリアルポートを指定
./target/release/security_camera_viewer --port /dev/ttyACM0

# 詳細ログを有効化
./target/release/security_camera_viewer --verbose
```

### MJPEGストリームとして保存

```bash
# デフォルト: output.mjpeg に保存
./target/release/security_camera_viewer

# カスタムファイル名
./target/release/security_camera_viewer --output my_video

# 再生
ffplay output.mjpeg
vlc output.mjpeg
```

### 個別JPEGファイルとして保存

```bash
# output/ ディレクトリに frame_000001.jpg, frame_000002.jpg, ... を保存
./target/release/security_camera_viewer --individual-files

# カスタムディレクトリ
./target/release/security_camera_viewer --individual-files --output frames/

# 閲覧
feh output/
eog output/
```

### オプション

| オプション | 説明 | デフォルト |
|----------|------|----------|
| `-p, --port <PORT>` | シリアルポートパス | 自動検出 |
| `-o, --output <OUTPUT>` | 出力先 (ファイル/ディレクトリ) | `output` |
| `--individual-files` | 個別JPEGファイルとして保存 | 無効 |
| `--max-frames <N>` | 最大フレーム数 (0=無制限) | 0 |
| `--max-errors <N>` | 最大連続エラー数 | 10 |
| `-v, --verbose` | 詳細ログ出力 | 無効 |
| `-l, --list` | 利用可能なポートを一覧表示 | - |

### 使用例

```bash
# 利用可能なシリアルポートを確認
./target/release/security_camera_viewer --list

# 100フレームだけキャプチャ
./target/release/security_camera_viewer --max-frames 100

# 個別JPEGファイルを詳細ログ付きで保存
./target/release/security_camera_viewer --individual-files --verbose
```

## 📊 プロトコル仕様

### MJPEGパケット構造

```
┌──────────┬──────────┬──────────┬───────────────┬──────────┐
│  HEADER  │   SEQ    │   SIZE   │  JPEG DATA    │ CHECKSUM │
│ (4 bytes)│ (4 bytes)│ (4 bytes)│  (variable)   │ (2 bytes)│
└──────────┴──────────┴──────────┴───────────────┴──────────┘
0xCAFEBABE  uint32     uint32      N bytes       CRC-16-CCITT
```

### パケット詳細

| フィールド | サイズ | 説明 |
|-----------|--------|------|
| SYNC_WORD | 4 bytes | 同期ワード (0xCAFEBABE) |
| SEQUENCE | 4 bytes | フレーム番号 |
| JPEG_SIZE | 4 bytes | JPEG データサイズ |
| JPEG_DATA | 可変 | JPEG 画像データ (SOI 0xFF 0xD8 ~ EOI 0xFF 0xD9) |
| CRC16 | 2 bytes | CRC-16-CCITT (ヘッダー + JPEG データ) |

### JPEG形式サポート

このビューアは以下のJPEG形式に対応しています:

- **JFIF形式**: `FF D8 FF E0` (JFIF APP0マーカー付き)
- **EXIF形式**: `FF D8 FF E1` (EXIF APP1マーカー付き)
- **ベアJPEG形式**: `FF D8 FF DB` (直接DQTマーカー) ← Spresense ISX012が出力

Spresense ISX012カメラはベアJPEG形式を出力します。これはJPEG標準に準拠した有効な形式で、ファイルサイズが小さく高速です。

### パフォーマンス

#### QVGA (320×240) - Phase 2.0
- **フレームレート**: 30 fps
- **平均JPEGサイズ**: 20.6 KB (ベアJPEG形式)
- **帯域使用率**: 4.9 Mbps (40.8% of USB 12 Mbps)
- **プロトコルオーバーヘッド**: 14 bytes (0.07%)

#### VGA (640×480) - Phase 3.0 🚀
- **Spresense 送信**: 30 fps (Phase 1.5 パイプライン: 37.33 fps 達成済み)
- **PC 受信 (Option A パイプライン)**:
  - **Before**: 16-20 fps (GUI スレッドでデコード)
  - **After**: **25-30 fps** (キャプチャスレッドでデコード、並列処理)
- **平均JPEGサイズ**: ~64 KB
- **帯域使用率**: 15.4 Mbps @ 30fps (128% of USB Full Speed 12 Mbps)
  - ※ USB High Speed (480 Mbps) 使用により問題なし
- **デコード時間**: 8-10 ms/frame (640×480 RGBA8)
- **シリアル読み込み時間**: 15-20 ms/frame (測定中)

## 🧪 テスト

```bash
# ユニットテストを実行
cargo test

# テスト結果: 7 passed
# - test_crc16_ccitt
# - test_invalid_sync_word
# - test_jpeg_size_limit
# - test_sync_word_validation
# - test_bare_jpeg_format  (NEW: ベアJPEG形式の検証)
# - test_jfif_jpeg_format  (NEW: JFIF形式の検証)
# - test_list_ports
```

## 🔧 トラブルシューティング

### Spresense が検出されない

```bash
# シリアルポートを確認
./target/release/security_camera_viewer --list

# 手動でポートを指定
./target/release/security_camera_viewer --port /dev/ttyACM0

# 権限を確認
sudo usermod -a -G dialout $USER
# ログアウト/ログインが必要
```

### JPEG エラーが発生する

```bash
# 詳細ログを有効化して確認
./target/release/security_camera_viewer --verbose

# バッファをフラッシュ後に再試行
# (自動的に実行されます)
```

### タイムアウトが頻発する

- USB ケーブルを確認
- Spresense が正しく動作しているか確認
- `--verbose` で詳細ログを確認

## 📖 関連ドキュメント

- **仕様書**: `/home/ken/Spr_ws/GH_wk_test/docs/security_camera/01_specifications/`
  - `04_MJPEG_PROTOCOL.md` - プロトコル詳細仕様
  - `06_SOFTWARE_SPEC_PC_RUST.md` - PC側ソフトウェア仕様
- **実装ガイド**: `/home/ken/Spr_ws/GH_wk_test/docs/case_study/13_PHASE2_RUST_GUIDE.md`

## 📜 ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。

## 🔍 実装詳細

### ファイル構成

```
src/
├── main.rs           # メインアプリケーション
├── protocol.rs       # MJPEG プロトコルパーサー
└── serial.rs         # USB CDC-ACM シリアル通信
```

### 依存クレート

- `serialport` - シリアルポート通信
- `image` - JPEG 画像処理
- `byteorder` - バイトオーダー変換
- `clap` - CLI 引数パース
- `anyhow` - エラーハンドリング
- `log` / `env_logger` - ロギング

### CRC-16-CCITT 実装

```rust
pub fn calculate_crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
```

---

## 📈 開発履歴

### Phase 3.0 (2025-12-31) 🚀
- **VGA (640×480) GUI ビューア対応**
- **Option A パイプライン実装**: JPEG デコードをキャプチャスレッドに移動
- **Windows クロスコンパイル対応**: MinGW-w64 によるネイティブ .exe ビルド
- **詳細性能メトリクス**: Serial/Decode/Texture 時間測定
- **性能改善**: 16-20 fps → 25-30 fps (VGA)

### Phase 2.0 (2025-12-22)
- **MJPEG プロトコル完全移行**: H.264 → MJPEG
- **ベアJPEG形式サポート**: Spresense ISX012 出力に対応
- **CRC-16-CCITT 実装**: プロトコルの信頼性向上
- **QVGA 30 fps 達成**: 4.9 Mbps 帯域使用率

### Phase 1.5 (2025-12-30)
- **Spresense VGA パイプライン**: カメラ+USB 並列処理
- **性能突破**: 9.94 fps → 37.33 fps (3.76倍改善)
- **マルチスレッド実装**: Priority 110 (Camera) + 100 (USB)

---

**作成日**: 2025-12-22
**最終更新**: 2025-12-31
**バージョン**: 3.0 (VGA + Option A パイプライン)
**ステータス**: ✅ 完成・Windows ビルド待ちテスト

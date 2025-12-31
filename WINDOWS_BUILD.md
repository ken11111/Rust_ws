# Windows向けクロスコンパイルガイド

**作成日**: 2025-12-30
**目的**: WSL2からWindows用の.exeファイルをビルドする

---

## 📋 前提条件

WSLgのOpenGL制約により、GUIアプリケーションをWSL2で実行できないため、Windows向けにクロスコンパイルします。

---

## 🔧 セットアップ手順

### Step 1: MinGW-w64のインストール

```bash
sudo apt-get update
sudo apt-get install -y mingw-w64
```

**確認**:
```bash
x86_64-w64-mingw32-gcc --version
```

期待される出力:
```
x86_64-w64-mingw32-gcc (GCC) X.X.X
```

---

### Step 2: Rustターゲットの追加

```bash
rustup target add x86_64-pc-windows-gnu
```

**確認**:
```bash
rustup target list | grep windows-gnu
```

期待される出力:
```
x86_64-pc-windows-gnu (installed)
```

---

### Step 3: Cargoの設定

Windows向けビルドのリンカー設定を追加:

```bash
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml << 'EOF'

[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"
EOF
```

**確認**:
```bash
cat ~/.cargo/config.toml
```

---

### Step 4: Windows向けビルド

```bash
cd /home/ken/Rust_ws/security_camera_viewer

# GUIアプリケーションをWindows向けにビルド
cargo build --release --target x86_64-pc-windows-gnu --features gui --bin security_camera_gui
```

**ビルド時間**: 初回は5-10分程度かかります（依存関係のコンパイル）

---

### Step 5: 実行ファイルの確認

```bash
ls -lh target/x86_64-pc-windows-gnu/release/security_camera_gui.exe
```

期待される出力:
```
-rwxr-xr-x 1 ken ken 15M Dec 30 13:30 security_camera_gui.exe
```

---

## 🚀 実行方法

### オプション1: WSLからWindowsエクスプローラーを開く

```bash
# Windowsエクスプローラーでフォルダを開く
explorer.exe target/x86_64-pc-windows-gnu/release/
```

→ `security_camera_gui.exe` をダブルクリックして実行

---

### オプション2: WSLからWindows実行ファイルを起動

```bash
# WSL2からWindows .exeを直接実行
./target/x86_64-pc-windows-gnu/release/security_camera_gui.exe
```

**注意**: Windows側でGUIが表示されます。

---

### オプション3: Windowsデスクトップにコピー

```bash
# Windowsのデスクトップにコピー（パスは環境に応じて変更）
cp target/x86_64-pc-windows-gnu/release/security_camera_gui.exe /mnt/c/Users/$(whoami)/Desktop/
```

---

## 📊 テスト実施

### 事前準備

1. **Spresense接続確認**:
   - Windowsデバイスマネージャーで「ポート (COM & LPT)」を開く
   - "USB Serial Device (COMx)" を確認
   - COMポート番号をメモ（例: COM3）

2. **GUIアプリケーション起動**:
   - `security_camera_gui.exe` を実行
   - GUIウィンドウが表示される

### 設定

1. **自動検出を無効化**:
   - 左パネルの "Auto-detect Spresense" のチェックを外す

2. **シリアルポートを設定**:
   - "Serial Port" 欄に `/dev/ttyACM0` と入力
   - （Windows側でCOM3の場合も、WSL2経由なので `/dev/ttyACM0` を使用）

3. **接続開始**:
   - "▶ Start" ボタンをクリック

### 性能測定

底部パネルの統計を記録:

| 項目 | 目標 | 測定値 |
|------|------|--------|
| **📊 FPS** | 30+ fps | _____ fps |
| **🎬 Frames** | カウントアップ | _____ |
| **❌ Errors** | 0 | _____ |
| **⏱ Decode** | <10 ms | _____ ms |

### 追加確認

- [ ] 映像がスムーズに表示される
- [ ] 解像度が "640x480" と表示される
- [ ] エラーが発生しない
- [ ] Windows上で問題なく動作する

---

## 🔄 再ビルド

コードを変更した後は:

```bash
cd /home/ken/Rust_ws/security_camera_viewer

# 再ビルド（増分ビルドなので高速）
cargo build --release --target x86_64-pc-windows-gnu --features gui --bin security_camera_gui

# 実行
./target/x86_64-pc-windows-gnu/release/security_camera_gui.exe
```

---

## ⚠️ トラブルシューティング

### 問題1: MinGW-w64がインストールできない

**エラー**:
```
E: Unable to locate package mingw-w64
```

**解決策**:
```bash
# リポジトリを更新
sudo apt-get update
sudo apt-get upgrade

# 再試行
sudo apt-get install -y mingw-w64
```

---

### 問題2: リンカーエラー

**エラー**:
```
error: linker `x86_64-w64-mingw32-gcc` not found
```

**解決策**:
```bash
# MinGW-w64が正しくインストールされているか確認
which x86_64-w64-mingw32-gcc

# インストールされていない場合
sudo apt-get install -y mingw-w64
```

---

### 問題3: ビルドが非常に遅い

**原因**: 初回ビルドは全ての依存関係をコンパイルするため時間がかかります。

**解決策**: 忍耐強く待つ（5-10分）。2回目以降は増分ビルドで高速化されます。

---

### 問題4: .exeが起動しない

**エラー**: "This app can't run on your PC"

**原因**: 32bit版のMinGWでビルドした可能性

**解決策**:
```bash
# 64bit版を明示的に指定
cargo build --release --target x86_64-pc-windows-gnu --features gui --bin security_camera_gui
```

---

### 問題5: シリアルポートが見つからない

**エラー**: "Failed to auto-detect" または "Permission denied"

**解決策**:

**方法1**: WSL2のUSB接続を確認
```bash
# WSL2でデバイスを確認
ls -l /dev/ttyACM0
```

**方法2**: Windows側のCOMポートを直接使用する場合
- 左パネルで "Auto-detect Spresense" のチェックを外す
- "Serial Port" に `COM3` などのWindowsポート名を入力
- ただし、WSL2経由では `/dev/ttyACM0` を使用する方が推奨

---

## 📝 ビルド設定の詳細

### Cargo.toml の確認

Windows向けビルドで必要な依存関係:

```toml
[dependencies]
# ... 他の依存関係 ...

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser", "combaseapi"] }
```

### ビルドフラグ

より小さい実行ファイルを生成:

```bash
# サイズ最適化ビルド
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-pc-windows-gnu --features gui --bin security_camera_gui
```

---

## 🎯 次のステップ

Windows向けビルドが成功したら:

1. **Step 1.3を完了**: VGA性能テスト実施
2. **Step 2に進む**: VGA統合動作テスト
3. **テスト結果を記録**: 性能データの収集

---

## 📚 参考情報

### クロスコンパイルの利点

- ✅ WSL2のOpenGL制約を回避
- ✅ Windowsネイティブ実行でパフォーマンス向上
- ✅ Windows環境での配布が容易
- ✅ GPU加速が利用可能（Windowsネイティブドライバー）

### 制限事項

- ⚠️ 初回ビルドに時間がかかる（5-10分）
- ⚠️ WSL2とWindows間のUSB接続設定が必要
- ⚠️ 実行ファイルサイズが大きい（15-20MB）

---

**作成者**: Claude Code (Sonnet 4.5)
**ステータス**: 📋 セットアップガイド完成

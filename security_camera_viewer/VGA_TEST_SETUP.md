# VGA表示性能テスト - セットアップガイド

**作成日**: 2025-12-30
**最終更新**: 2025-12-31 (Option A パイプライン対応)
**目的**: Phase 3.0 - VGA (640×480) GUI ビューア性能テスト（パイプライン最適化版）

---

## 🚀 Option A パイプライン実装完了 (2025-12-31)

**重要な変更**:
- JPEG デコードをキャプチャスレッドに移動（GUI スレッドから分離）
- シリアル読み込み・デコード・テクスチャ時間の詳細測定
- 期待性能: **16-20 fps → 25-30 fps**

**推奨テスト方法**: Windows クロスコンパイル版（下記参照）

---

## 📋 前提条件

### Spresense側
- ✅ VGAファームウェア（30 fps 連続送信版）がフラッシュ済み
  - Phase 1.5 パイプライン: 37.33 fps 達成版
  - 連続送信モード（Ctrl+C で停止）
- ✅ USB CDC-ACM接続が動作
- ✅ `/dev/ttyACM0` (WSL2) または COM4 (Windows) デバイスが認識されている

### PC側
**Option 1: Windows クロスコンパイル版（推奨）**:
- ✅ MinGW-w64 インストール済み
- ✅ Rust target `x86_64-pc-windows-gnu` 追加済み
- ✅ Option A パイプライン実装版ビルド済み
- 詳細: [WINDOWS_BUILD.md](WINDOWS_BUILD.md)

**Option 2: WSL2 版**:
- ✅ X11サーバーがWindows上で動作中（WSLg または VcXsrv）
- ⚠️ 性能制限あり（ソフトウェアレンダリング）

---

## 🖥️ X11サーバーのセットアップ

### Windows 11の場合（推奨）

Windows 11にはWSLg（WSL GUI）が統合されており、追加のセットアップは**不要**です。

**確認方法**:
```bash
# WSL2で実行
echo $DISPLAY
# 出力例: :0 または :1
```

出力があれば、WSLgが動作しています。

---

### Windows 10の場合

X11サーバー（VcXsrvまたはX410）をインストールする必要があります。

#### オプション1: VcXsrv（無料、推奨）

1. **ダウンロード**:
   https://sourceforge.net/projects/vcxsrv/

2. **インストール**:
   - `vcxsrv-64.x.x.x.installer.exe` を実行
   - デフォルト設定でインストール

3. **起動**:
   - `XLaunch` を起動
   - **Display settings**: "Multiple windows" を選択
   - **Display number**: 0（デフォルト）
   - **Start no client** を選択
   - **Extra settings**: "Disable access control" をチェック ✅
   - "Finish" をクリック

4. **ファイアウォール許可**:
   - Windowsセキュリティの警告が出たら「アクセスを許可する」

5. **WSL2側の設定**:
   ```bash
   export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0
   ```

#### オプション2: X410（有料、$9.99）

Microsoft Storeから入手:
https://www.microsoft.com/store/apps/9NLP712ZMN9Q

設定は簡単ですが、有料です。

---

## 🧪 テスト手順

### Step 1: X11接続確認

```bash
cd /home/ken/Rust_ws/security_camera_viewer

# X11が動作しているか確認
xdpyinfo | head -10
```

**成功例**:
```
name of display:    :0
version number:     11.0
vendor string:      Microsoft Corporation
...
```

**失敗例**:
```
xdpyinfo: unable to open display ":0"
```

→ X11サーバーが起動していないか、DISPLAYが正しく設定されていません。

---

### Step 2: Spresense接続確認

```bash
# USB CDC-ACMデバイスの確認
ls -l /dev/ttyACM*

# 期待される出力:
# crw-rw---- 1 root dialout 166, 0 Dec 30 10:00 /dev/ttyACM0
```

デバイスが見つからない場合:
```bash
# WSL2でUSB接続を確認
lsusb | grep -i sony
# または
dmesg | grep -i tty
```

---

### Step 3: GUIビューア起動

```bash
cd /home/ken/Rust_ws/security_camera_viewer

# テスト実行
./run_gui.sh
```

**正常起動時の出力**:
```
Checking X11 connection...

Starting Spresense Security Camera GUI...
DISPLAY: :0
Note: Using software rendering (may be slower in WSL2)
```

GUIウィンドウが表示されます。

---

### Step 4: VGA性能測定

#### 測定項目（Option A パイプライン版）

GUIウィンドウの底部パネルで以下を確認:

| 項目 | 目標 | 測定値 | 備考 |
|------|------|--------|------|
| **📊 FPS** | 25-30 fps | _____ fps | Option A 目標 |
| **🎬 Frames** | カウントアップ | _____ | 正常動作確認 |
| **❌ Errors** | 0 | _____ | エラーなし期待 |
| **⏱ Decode** | 8-10 ms | _____ ms | キャプチャスレッドで測定 |
| **📡 Serial** | 15-20 ms | _____ ms | シリアル読み込み時間 |
| **🖼 Texture** | 2-3 ms | _____ ms | GUI スレッド負荷 |

#### 測定手順

1. **Startボタンをクリック**
   - Status が "Connected" になることを確認

2. **30秒間観察**
   - FPSが安定するまで待つ（最初の5秒は不安定な場合あり）
   - 底部パネルの統計を記録

3. **Stopボタンをクリック**
   - エラーカウントを確認

4. **再度Startで繰り返し**
   - 3回測定して平均を取る

#### 追加測定（オプション）

**CPU使用率**:
```bash
# 別のWSL2ターミナルで実行
top -p $(pgrep -f security_camera_gui)
```

**メモリ使用量**:
```bash
ps aux | grep security_camera_gui | grep -v grep | awk '{print $6/1024 " MB"}'
```

---

## 📊 期待される性能

### VGA (640×480) Option A パイプライン版の目標値

| 項目 | Before (単一スレッド) | After (Option A) | 根拠 |
|------|---------------------|------------------|------|
| **表示FPS** | 16-20 fps | **25-30 fps** | JPEG デコード並列処理 |
| **デコード時間** | 8-10 ms (GUI) | 8-10 ms (キャプチャ) | 同じだが並列実行 |
| **GUI スレッド負荷** | デコード+テクスチャ | テクスチャのみ (2-3ms) | 負荷削減 |
| **シリアル読み込み** | 測定なし | 15-20 ms | 新規測定 |
| **CPU使用率** | <50% | <40% | 負荷分散 |
| **メモリ使用量** | <100 MB | <120 MB | RGBA バッファ増加 |
| **エラー率** | 0% | 0% | USB通信の信頼性 |

**性能改善の理由**:
- **Before**: GUI スレッドでデコード (8-10ms) + テクスチャ (2-3ms) = **10-13ms** 直列処理
- **After**: キャプチャスレッドでデコード (8-10ms) || GUI スレッドでテクスチャ (2-3ms) = **並列実行**
- **結果**: GUI の 60 FPS レンダリングサイクルを維持しながら、キャプチャスレッドが並列デコード

### QVGAとの比較

| 項目 | QVGA (320×240) | VGA (640×480) | 増加率 |
|------|----------------|---------------|--------|
| ピクセル数 | 76,800 | 307,200 | **4倍** |
| JPEG平均サイズ | ~20 KB | ~64 KB | **3.2倍** |
| 帯域幅 | 4.8 Mbps | 15.4 Mbps @ 30fps | **3.2倍** |
| デコード時間（推定） | ~3 ms | ~8-10 ms | **3倍** |

---

## ⚠️ トラブルシューティング

### 問題1: GUIウィンドウが表示されない

**原因**: X11サーバーが起動していない

**解決策**:
1. Windows側でVcXsrvまたはX410を起動
2. WSL2で `export DISPLAY=:0` を実行
3. `xdpyinfo` で接続確認

---

### 問題2: "unable to open display" エラー

**エラーメッセージ**:
```
Error: X11Error: ConnectionRefused
```

**解決策**:
```bash
# DISPLAYを再設定（VcXsrvの場合）
export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0

# 確認
echo $DISPLAY
xdpyinfo
```

---

### 問題3: FPSが低い（<15 fps）

**原因**: ソフトウェアレンダリングの制限、またはUSB通信の問題

**解決策**:
1. **GPU加速が無効か確認**:
   ```bash
   echo $LIBGL_ALWAYS_SOFTWARE
   # 出力: 1 (ソフトウェアレンダリング使用中)
   ```

2. **USB接続を確認**:
   ```bash
   # USBエラーがないか確認
   dmesg | tail -50 | grep -i "cdc_acm\|ttyACM"
   ```

3. **Spresense側の出力を確認**:
   ```bash
   # 別ターミナルでシリアルログ確認
   screen /dev/ttyACM0 115200
   ```

---

### 問題4: "Permission denied" エラー

**エラーメッセージ**:
```
Error: Permission denied (os error 13)
```

**解決策**:
```bash
# dialoutグループに追加
sudo usermod -a -G dialout $USER

# ログアウト/ログインが必要
exit
# WSL2を再起動してログイン

# 確認
groups | grep dialout
```

---

### 問題5: WSLgが動作しない（Windows 11）

**確認**:
```bash
wsl --status
# WSL version: 2.x.x.x
# Kernel version: 5.x.x
```

**解決策**:
```bash
# WSLを最新版に更新（Windows側のPowerShellで実行）
wsl --update
wsl --shutdown

# WSL2を再起動
```

---

## 📝 テスト結果記録テンプレート

### テスト環境

- **日時**: YYYY-MM-DD HH:MM
- **Windows版**: Windows 10/11
- **WSLバージョン**: `wsl --version` の出力
- **X11サーバー**: WSLg / VcXsrv / X410
- **Spresenseファームウェア**: VGA 37.33 fps版

### 測定結果

#### 試行1
- FPS: _____ fps
- Decode: _____ ms
- Errors: _____
- CPU使用率: _____ %
- メモリ: _____ MB

#### 試行2
- FPS: _____ fps
- Decode: _____ ms
- Errors: _____
- CPU使用率: _____ %
- メモリ: _____ MB

#### 試行3
- FPS: _____ fps
- Decode: _____ ms
- Errors: _____
- CPU使用率: _____ %
- メモリ: _____ MB

### 平均値
- **平均FPS**: _____ fps
- **平均Decode時間**: _____ ms
- **総エラー数**: _____
- **平均CPU使用率**: _____ %
- **平均メモリ使用量**: _____ MB

### 評価

- [ ] FPS ≥ 30 fps
- [ ] Decode時間 < 10 ms
- [ ] エラー = 0
- [ ] CPU使用率 < 30%
- [ ] メモリ使用量 < 100 MB

### コメント

（気づいた点、問題点、改善案など）

---

## 🚀 次のステップ

テストが成功したら:
1. **Step 1.3を完了**としてマーク
2. **Step 2: VGA統合動作テスト**に進む
3. テスト結果をドキュメント化

テストが失敗したら:
1. トラブルシューティングセクションを参照
2. 問題を特定して修正
3. 再テスト

---

**作成者**: Claude Code (Sonnet 4.5)
**ステータス**: 📋 テスト準備完了

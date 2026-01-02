# Windows版リリースガイド

## ビルド完了

Windows用の実行ファイル（.exe）のビルドが完了しました。

---

## 📦 リリースパッケージの内容

### ディレクトリ構成

```
release/windows/
├── security_camera_gui.exe      (16MB) - メインアプリケーション
├── README.txt                   (4.8KB) - 詳細マニュアル
├── QUICKSTART.txt               (2.8KB) - クイックスタートガイド
├── VERSION.txt                  (4.3KB) - バージョン情報
└── run.bat                      (363B)  - 起動用バッチファイル
```

### アーカイブファイル

```
release/security_camera_viewer_phase5_windows.tar.gz (5.9MB)
```

---

## 🪟 Windowsからのアクセス方法

### 方法1: Explorerから直接アクセス

WSL2のファイルシステムはWindowsのエクスプローラーからアクセスできます:

1. Windowsのエクスプローラーを開く
2. アドレスバーに以下を入力:
   ```
   \\wsl$\Ubuntu\home\ken\Rust_ws\security_camera_viewer\release\windows
   ```
3. Enter キーを押す
4. ファイルが表示されます

### 方法2: コマンドプロンプトからコピー

```cmd
xcopy \\wsl$\Ubuntu\home\ken\Rust_ws\security_camera_viewer\release\windows C:\SecurityCamera\ /E /I
```

### 方法3: WSL内でWindows側にコピー

WSL内から以下のコマンドでWindows側にコピー:

```bash
cd /home/ken/Rust_ws/security_camera_viewer/release
cp -r windows /mnt/c/Users/%USERNAME%/Desktop/SecurityCamera
```

または特定のフォルダに:

```bash
mkdir -p /mnt/c/SecurityCamera
cp -r windows/* /mnt/c/SecurityCamera/
```

---

## 🚀 配布方法

### 配布用パッケージの作成

圧縮ファイルを配布する場合:

```bash
# 現在のディレクトリ: /home/ken/Rust_ws/security_camera_viewer/release

# tar.gz形式（既に作成済み）
ls -lh security_camera_viewer_phase5_windows.tar.gz
# 5.9MB
```

### Windows側で展開

Windows側でtar.gzを展開するには:
- 7-Zip (https://www.7-zip.org/)
- WinRAR
- Windows 11標準のtar機能

---

## 📝 使用方法（エンドユーザー向け）

### 必要なもの

1. **Windows 10/11 (64bit)**
2. **Spresenseカメラデバイス**（USB接続）
3. **VLC Media Player**（録画ファイル再生用）
   - ダウンロード: https://www.videolan.org/vlc/

### 起動方法

**方法A: バッチファイルから起動（推奨）**
1. `run.bat` をダブルクリック
2. 自動的にrecordingsフォルダが作成され、アプリが起動

**方法B: 直接起動**
1. `security_camera_gui.exe` をダブルクリック
2. GUIウィンドウが開く

### 初回起動時

Visual C++ Redistributable が必要な場合があります:
- エラーが出た場合、以下からダウンロード:
  https://aka.ms/vs/17/release/vc_redist.x64.exe

---

## 🎯 動き検知録画の使い方

### クイックスタート

1. アプリ起動後、Spresenseを接続
2. シリアルポート選択（例: COM3）
3. "Connect" ボタンクリック
4. 左パネルの "🔍 Motion Detection" セクション
5. "Enable Motion Detection" にチェック
6. 10秒待つ（バッファ蓄積）
7. カメラの前で動く → 自動録画開始！

### 録画ファイルの場所

```
実行ファイルと同じフォルダ/recordings/motion_YYYYMMDD_HHMMSS.mjpeg
```

例:
```
C:\SecurityCamera\recordings\motion_20260102_143045.mjpeg
```

---

## 🔧 トラブルシューティング

### 問題: アプリが起動しない

**解決策**:
1. Visual C++ Redistributable をインストール
2. Windows Defenderの除外設定を追加

### 問題: カメラ映像が表示されない

**解決策**:
1. デバイスマネージャーでCOMポートを確認
2. Spresenseを再接続
3. ドライバーが正しくインストールされているか確認

### 問題: 録画ファイルが再生できない

**解決策**:
1. VLC Media Player をインストール
2. ファイルを右クリック → "プログラムから開く" → VLC

---

## 📊 パフォーマンス情報

### システム要件

**最小要件**:
- OS: Windows 10 (64bit)
- RAM: 512MB
- ディスク: 1GB（録画用）

**推奨要件**:
- OS: Windows 11 (64bit)
- RAM: 1GB以上
- ディスク: 10GB以上（SSD推奨）

### メモリ使用量

- ベース: 約200MB
- 動き検知ON: +6.7MB（合計約207MB）

### 録画ファイルサイズ

- 1分間: 約33MB
- 10分間: 約330MB
- 1時間: 約2GB（制限により自動分割）

---

## 📄 ドキュメントファイル

リリースパッケージに含まれるドキュメント:

1. **README.txt**: 詳細マニュアル（トラブルシューティング含む）
2. **QUICKSTART.txt**: クイックスタートガイド
3. **VERSION.txt**: バージョン情報と変更履歴

追加ドキュメント（開発用）:
- `MOTION_DETECTION_TEST_PLAN.md`: テスト計画書
- `PHASE5_IMPLEMENTATION_SUMMARY.md`: 実装サマリー
- `PHASE5_MOTION_DETECTION_SPEC.md`: 詳細仕様書

---

## 🎉 次のステップ

### エンドユーザー向け

1. アプリを起動してカメラを接続
2. 動き検知録画を試す
3. 設定を調整して最適化
4. 長時間運用テスト

### 開発者向け

1. 実機テストの実施
2. フィードバック収集
3. Phase 6実装（MP4録画対応）
4. バグ修正とパフォーマンス改善

---

## 📦 配布チェックリスト

配布前に確認:
- [ ] README.txt が最新
- [ ] VERSION.txt にビルド日が記載
- [ ] .exe ファイルが起動できる（Windows実機で確認）
- [ ] 録画機能が動作する
- [ ] VLCで録画ファイルが再生できる
- [ ] ウイルススキャン実施（誤検知対策）

---

## 🔐 セキュリティ考慮事項

### ウイルススキャン

配布前に以下でスキャン推奨:
- Windows Defender
- VirusTotal (https://www.virustotal.com/)

### デジタル署名

本リリースは未署名です。
商用配布する場合は、コード署名証明書の取得を推奨します。

---

## 📞 サポート情報

### 問題報告時に必要な情報

1. Windowsバージョン（`winver` コマンドで確認）
2. エラーメッセージ（スクリーンショット）
3. 実行したい操作
4. Spresenseデバイス情報

### ログの確認

アプリケーションログは標準出力に表示されます。
`run.bat` から起動するとコンソールウィンドウでログが確認できます。

---

## ✅ ビルド情報

**ビルド日**: 2026-01-02
**ビルド環境**: WSL2 (Ubuntu) + MinGW-w64
**コンパイラ**: x86_64-w64-mingw32-gcc
**Rustバージョン**: 2021 edition
**最適化レベル**: --release

**ファイルサイズ**:
- security_camera_gui.exe: 16MB
- tar.gz圧縮後: 5.9MB（圧縮率63%）

---

## 🚀 今後の予定

### Phase 6: MP4録画対応
- ffmpeg経由でのMP4エンコード
- ファイルサイズ約50%削減
- 実装予定: 2026年1月

### Phase 7: 長時間運用
- 24時間連続録画対応
- 古いファイル自動削除
- ディスク容量監視

---

**開発**: Claude Sonnet 4.5 + User
**最終更新**: 2026-01-02

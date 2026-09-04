# MiniVisor ビルド・実行テスト

## 🎯 目的
Type-1ハイパーバイザーとしてのMiniVisorを実際にビルドし、ベアメタル実行を体験する

## 📋 前提条件確認

### 必要なツール
- Rust toolchain (aarch64-unknown-none-softfloat)
- QEMU (ARM64 emulation)
- dtc (Device Tree Compiler)

### 設定確認
```toml
# Cargo.toml での設定
target = "aarch64-unknown-none-softfloat"  # ベアメタル用ターゲット
rustflags = ["-C", "link-arg=-Tscripts/qemu.ld"]  # 専用リンカスクリプト
```

## 🔍 ベアメタル証拠の技術確認

### 1. ターゲット設定の意味
```
aarch64-unknown-none-softfloat
├─ aarch64: ARM64アーキテクチャ
├─ unknown: ベンダー不明（汎用）
├─ none: OSなし（ベアメタル）
└─ softfloat: ソフトウェア浮動小数点
```

### 2. リンカスクリプト分析
```bash
# scripts/qemu.ld を確認
# - 物理アドレス直接指定
# - メモリレイアウト定義
# - エントリポイント設定
```

### 3. no_std/no_main の効果
```rust
#![no_std]   // libc、標準ライブラリ不使用
#![no_main]  // 通常のmain()関数不使用
// → OS依存を完全排除
```

## 🧪 実際のテスト手順

### Step 1: ビルド環境確認
```bash
# Rust toolchain確認
rustc --version
rustup target list --installed | grep aarch64

# QEMU確認（ARM64サポート）
qemu-system-aarch64 --version

# Device Tree Compiler確認
dtc --version
```

### Step 2: MiniVisorビルド
```bash
cd /home/ken/Rust_ws/hypervisor_study/MiniVisor
cargo build --release
```

### Step 3: 実行テスト
```bash
# QEMUでの実行
cargo run --release
# または
tools/run_qemu.sh target/aarch64-unknown-none-softfloat/release/mini_visor
```

## 📊 期待される結果

### 正常ビルド時の出力
```
Compiling mini_visor v1.0.0 (...)
Finished release [optimized] target(s) in X.XXs
```

### 正常実行時の出力
```
Hello, world!
CurrentEL: 2           ← EL2での実行確認
[初期化メッセージ群]
```

## 🔍 学習ポイント

### 1. ベアメタルバイナリの生成確認
- OSに依存しない実行ファイル生成
- 専用リンカスクリプトによるメモリレイアウト制御
- ファームウェアから直接実行可能な形式

### 2. EL2での実行確認
- CurrentEL レジスタでの特権レベル確認
- ハイパーバイザー特権での動作実証

### 3. 物理ハードウェア制御の確認
- メモリ管理の初期化
- 割り込みコントローラー設定
- デバイスの直接制御

## 🚨 トラブルシューティング

### よくある問題

1. **ツールチェーン不足**
   ```bash
   rustup target add aarch64-unknown-none-softfloat
   ```

2. **QEMU不足**
   ```bash
   # Ubuntu/Debian
   sudo apt install qemu-system-arm

   # macOS
   brew install qemu
   ```

3. **dtc不足**
   ```bash
   # Ubuntu/Debian
   sudo apt install device-tree-compiler
   ```

## 📝 学習成果確認

### 理解度チェック
- [ ] ベアメタル実行の意味を技術的に説明できる
- [ ] MiniVisorがType-1である理由をビルド設定で説明できる
- [ ] EL2実行の重要性を理解している

### 実践確認
- [ ] MiniVisorのビルドに成功した
- [ ] QEMUでの実行ができた
- [ ] CurrentEL: 2 の出力を確認した
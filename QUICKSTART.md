# Security Camera Viewer - クイックスタート

Phase 2 (PC側Rust実装) を開始するための手順書

---

## 🚀 5分でスタート

### 1. プロジェクト作成
```bash
cd /home/ken/Rust_ws
cargo new security_camera_viewer --bin
cd security_camera_viewer
```

### 2. 依存関係追加

`Cargo.toml` を以下の内容に置き換え:

```toml
[package]
name = "security_camera_viewer"
version = "0.1.0"
edition = "2021"

[dependencies]
serialport = "4.5"
bytes = "1.5"
byteorder = "1.5"
crc = "3.0"
tokio = { version = "1.35", features = ["full"] }
log = "0.4"
env_logger = "0.11"
anyhow = "1.0"
thiserror = "1.0"
clap = { version = "4.4", features = ["derive"] }
```

### 3. 最小限の動作確認

`src/main.rs`:

```rust
use clap::Parser;
use log::info;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    port: Option<String>,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    info!("Security Camera Viewer v0.1.0");
    info!("Port: {:?}", args.port.unwrap_or_else(|| "auto".to_string()));

    // シリアルポート一覧表示
    if let Ok(ports) = serialport::available_ports() {
        info!("Available ports:");
        for p in ports {
            info!("  - {}", p.port_name);
        }
    }
}
```

### 4. ビルド・実行
```bash
cargo build
cargo run -- --port /dev/ttyACM0
```

---

## 📂 次のステップ

詳細な実装は `/home/ken/Spr_ws/case_study/13_PHASE2_RUST_GUIDE.md` を参照。

### 実装順序:
1. ✅ プロジェクトセットアップ (上記)
2. ⬜ プロトコル定義 (src/protocol.rs)
3. ⬜ シリアル通信 (src/serial.rs)
4. ⬜ メイン処理 (src/main.rs)
5. ⬜ 統合テスト

---

## 🔧 トラブルシューティング

### シリアルポートが見つからない
```bash
# デバイス確認
ls -l /dev/ttyACM*

# Spresense確認
lsusb | grep 054c

# パーミッション設定
sudo usermod -a -G dialout $USER
# ログアウト・ログイン
```

### Rust環境がない
```bash
# Rustインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

**参照**: `/home/ken/Spr_ws/case_study/13_PHASE2_RUST_GUIDE.md`

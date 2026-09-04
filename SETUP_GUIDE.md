# 再対策案 セットアップガイド

## 📂 作成されたディレクトリ構成

```
/home/ken/Rust_ws/
├── security_camera_viewer/          # 既存プロジェクト（拡張済み）
│   ├── .github/
│   │   └── workflows/
│   │       └── ci.yml               ✅ NEW: GitHub Actions CI/CD設定
│   ├── benches/
│   │   └── performance.rs           ✅ NEW: 性能ベンチマークテスト
│   ├── AUTOMATION_IMPLEMENTATION.md ✅ NEW: 検証自動化実装ガイド
│   └── ... (既存ファイル)
│
└── rust-learning-journey/           ✅ NEW: Rust学習記録リポジトリ
    ├── README.md                    # 学習計画・進捗記録
    ├── IMPLEMENTATION_GUIDE.md      # 実装ガイド（優先度・期限）
    │
    ├── rust-book-notes/             # The Rust Book学習メモ
    │   ├── ch04-ownership.md        ✅ テンプレート作成済み
    │   └── code-examples/
    │
    ├── leetcode-solutions/          # LeetCode解答コード
    │   ├── easy/
    │   │   ├── README.md            # 進捗管理
    │   │   └── two_sum.rs           ✅ サンプル実装済み
    │   └── medium/
    │
    ├── embedded-rust/               # 組込みRustプロジェクト
    │   ├── led-blink/
    │   │   └── README.md            ✅ 実装計画作成済み
    │   ├── uart-echo/
    │   └── timer-interrupt/
    │
    └── mini-projects/               # 小規模プロジェクト
        ├── cli-tool/
        └── async-tcp/
```

---

## 🚀 セットアップ手順

### 1. security_camera_viewer の拡張

#### Step 1: Cargo.toml にベンチマーク設定追加
```bash
cd /home/ken/Rust_ws/security_camera_viewer
```

以下を `Cargo.toml` に追加:
```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "performance"
harness = false
```

#### Step 2: benches/performance.rs を実際のコードに合わせて修正
```rust
// src/protocol.rs の実際の関数を使用
use security_camera_viewer::protocol::calculate_crc16_ccitt;

// ベンチマーク実装を実際のコードに合わせる
```

#### Step 3: GitHub Actions 動作確認
```bash
# Gitコミット
git add .github/workflows/ci.yml benches/ AUTOMATION_IMPLEMENTATION.md
git commit -m "feat: Add CI/CD, benchmarks, and automation guide"

# プッシュ（リモートリポジトリがある場合）
git push origin main

# GitHub上でActions実行を確認
```

#### Step 4: README.md にバッジ追加
```markdown
# Security Camera Viewer

![CI](https://github.com/YOUR_USERNAME/security_camera_viewer/workflows/CI%2FCD/badge.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
```

---

### 2. rust-learning-journey の初期化

#### Step 1: Gitリポジトリ初期化
```bash
cd /home/ken/Rust_ws/rust-learning-journey

git init
git add .
git commit -m "Initial commit: Rust learning journey structure"
```

#### Step 2: GitHub リポジトリ作成・プッシュ
```bash
# GitHub上で新規リポジトリ作成: rust-learning-journey

git remote add origin https://github.com/YOUR_USERNAME/rust-learning-journey.git
git branch -M main
git push -u origin main
```

#### Step 3: 学習開始日を記録
```bash
# README.md の「作成日」を更新
# 学習時間記録テーブルを更新
```

---

### 3. LeetCode 解答の実装

#### Step 1: Two Sum問題を実際に解く
```bash
cd /home/ken/Rust_ws/rust-learning-journey/leetcode-solutions/easy

# two_sum.rs を編集
# テスト実行
rustc --test two_sum.rs && ./two_sum

# またはプロジェクト化
cargo new --lib leetcode_easy
# Cargo.toml設定後
cargo test
```

#### Step 2: LeetCode上で提出
1. https://leetcode.com/problems/two-sum/
2. Rust言語を選択
3. コードをコピー＆ペースト
4. Submit
5. 実行時間・メモリ使用量を記録

#### Step 3: README.md 更新
```markdown
| 1 | Two Sum | Easy | [two_sum.rs](./two_sum.rs) | 0ms | 2.1MB | 2026-03-15 |
```

---

### 4. 組込みRust環境構築（オプション）

#### ハードウェア推奨
- **STM32 Blue Pill** ($3程度、Amazon/AliExpress)
- **ST-Link V2** ($2程度、デバッガ）

#### ソフトウェアセットアップ
```bash
# Rust embedded toolchain
rustup target add thumbv7m-none-eabi

# ツールインストール
cargo install cargo-embed
cargo install cargo-binutils
rustup component add llvm-tools-preview
```

---

## ✅ 実装チェックリスト（2週間計画）

### Week 1: 最優先タスク
- [ ] Day 1: GitHub Actions CI/CD構築完了
- [ ] Day 2: rust-learning-journey リポジトリ公開
- [ ] Day 3-5: LeetCode Easy 10問完了
- [ ] Day 6-7: ベンチマークテスト実装・動作確認

### Week 2: 重要タスク
- [ ] Day 8-10: LeetCode Easy 残り10問完了
- [ ] Day 11-12: 経歴書修正、面接回答準備
- [ ] Day 13-14: 組込みRust環境構築（可能なら）

### 応募判断（Day 14終了時点）
```yaml
最低条件達成確認:
  ✅ GitHub Actions CI/CD動作: Yes/No
  ✅ rust-learning-journey公開: Yes/No
  ✅ LeetCode 20問完了: Yes/No
  ✅ 経歴書修正完了: Yes/No
  ✅ 面接回答準備: Yes/No

判断:
  - 全てYes → 応募可能
  - 1つでもNo → 1週間延長検討
```

---

## 📊 進捗管理

### 毎日の習慣
1. **学習時間記録**: rust-learning-journey/README.md更新
2. **Gitコミット**: 学習成果を毎日コミット
3. **進捗確認**: チェックリスト更新

### 週次レビュー
1. **学習時間集計**: 週20時間目標
2. **完了タスク確認**: 遅れがあれば計画調整
3. **次週計画**: 優先度再確認

---

## 🔗 関連ドキュメント

- [実装ガイド](./rust-learning-journey/IMPLEMENTATION_GUIDE.md)
- [検証自動化ガイド](./security_camera_viewer/AUTOMATION_IMPLEMENTATION.md)
- [学習計画](./rust-learning-journey/README.md)

---

## 💡 よくある質問

### Q1: ハードウェアを購入する必要がありますか？
A: 必須ではありません。最低条件（2週間後応募）にはソフトウェア実装のみで対応可能です。ハードウェアは理想条件（1ヶ月後）での追加要素です。

### Q2: LeetCode有料版が必要ですか？
A: 不要です。無料問題のみで十分です。

### Q3: GitHub Actions の費用は？
A: パブリックリポジトリなら無料です。

### Q4: どれくらいの時間が必要ですか？
A: 最低条件達成には **週20時間 × 2週間 = 40時間** 程度を想定しています。

---

**作成日**: 2026-03-15
**最終更新**: 2026-03-15

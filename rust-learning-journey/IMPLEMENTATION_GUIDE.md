# 実装ガイド - 再対策案

## 🎯 目的
求人応募に向けた実装実績の作成と、技術力の実証

## 📋 優先度別タスク

### 🔴 最優先（応募前必須 - 2週間以内）

#### 1. GitHub Actions CI/CD構築 ✅テンプレート作成済み
**ファイル**: `security_camera_viewer/.github/workflows/ci.yml`

**実装ステップ**:
```bash
cd /home/ken/Rust_ws/security_camera_viewer

# 1. Cargo.tomlにベンチマーク設定追加
# 2. benches/performance.rsを実際のコードに合わせて修正
# 3. gitコミット・プッシュ
git add .github/workflows/ci.yml benches/
git commit -m "feat: Add GitHub Actions CI/CD and performance benchmarks"
git push origin main

# 4. GitHub Actionsの実行確認
# 5. README.mdにバッジ追加
```

**完了基準**:
- [ ] CI/CDが正常に動作（グリーンチェック）
- [ ] README.mdにバッジ表示
- [ ] ベンチマーク結果が記録される

#### 2. Rust学習記録の公開
**ディレクトリ**: `rust-learning-journey/`

**実装ステップ**:
```bash
cd /home/ken/Rust_ws/rust-learning-journey

# 1. Gitリポジトリ初期化
git init
git add .
git commit -m "Initial commit: Rust learning journey structure"

# 2. GitHubにプッシュ（新規リポジトリ作成）
# GitHub上で rust-learning-journey リポジトリ作成後
git remote add origin https://github.com/YOUR_USERNAME/rust-learning-journey.git
git push -u origin main

# 3. README.md更新（学習開始日記録）
# 4. 週次で進捗更新
```

**完了基準**:
- [ ] GitHubリポジトリ公開
- [ ] README.mdに学習計画記載
- [ ] 最低3件の学習記録（LeetCode解答/学習ノート）

#### 3. C言語コーディング練習
**目的**: コーディングテスト対策

**実装ステップ**:
```bash
# 1. LeetCode Easy問題 20問（Rust/C両方で解く）
# 2. アルゴリズム復習（ソート、探索、木構造）
# 3. 組込み特有問題（ビット操作、メモリ効率）
```

**完了基準**:
- [ ] LeetCode Easy 20問完了（Rustで解答）
- [ ] 解答コードをrust-learning-journeyに記録
- [ ] 実行時間・メモリ使用量を記録

---

### 🟡 重要（1-2週間以内）

#### 4. 経歴書のRust表現修正
**ファイル**: 職務経歴書（既存）

**修正ポイント**:
- [ ] 「AI協働による学習プロジェクト」と明記
- [ ] 「実装はAI支援、設計は自身で実施」と正直に記載
- [ ] 「業務ではC言語をメイン想定」と明確化
- [ ] プライベート研究と商用開発の違いを認識している旨を記載

#### 5. 面接想定質問への回答準備
**ファイル**: `INTERVIEW_PREP.md`（作成推奨）

**準備内容**:
- [ ] 「Rustは自分で書けますか？」への回答
- [ ] 「チーム開発経験は？」への回答
- [ ] 「検証自動化の経験は？」への回答
- [ ] 「AI協働とは何ですか？」への回答

---

### 🟢 推奨（1ヶ月以内）

#### 6. OSSコントリビューション
**目標**: PR 2-3件

**候補プロジェクト**:
- 組込みRust関連（embedded-hal等）
- Rustツール関連（cargo-xxx等）
- ドキュメント修正からスタート

**実装ステップ**:
```bash
# 1. Good First Issue探し
# 2. フォーク・ブランチ作成
# 3. 修正・テスト
# 4. Pull Request作成
```

#### 7. 組込みRust小規模プロジェクト
**ディレクトリ**: `rust-learning-journey/embedded-rust/`

**実装ステップ**:
1. ハードウェア入手（STM32 Blue Pill推奨、$3程度）
2. LED点滅実装
3. UART通信実装
4. タイマー割り込み実装

---

## 📊 進捗管理

### Week 1-2（最優先）
```yaml
Day 1-2:
  - GitHub Actions CI/CD構築
  - rust-learning-journeyリポジトリ作成
  - README.md更新

Day 3-5:
  - LeetCode Easy 10問
  - 学習ノート作成開始

Day 6-7:
  - ベンチマークテスト実装
  - CI/CD動作確認

Day 8-14:
  - LeetCode Easy 残り10問
  - 組込みRust環境構築開始
```

### Week 3-4（重要）
```yaml
Day 15-21:
  - 経歴書修正
  - 面接想定質問回答準備
  - 組込みRustプロジェクト実装

Day 22-28:
  - OSSコントリビューション探し
  - 技術勉強会参加検討
  - ポートフォリオ整理
```

---

## ✅ 応募判断チェックリスト

### 最低条件（2週間後）
- [ ] GitHub Actions CI/CD完成・動作確認
- [ ] rust-learning-journeyリポジトリ公開（進捗20%以上）
- [ ] LeetCode Easy 20問完了
- [ ] 経歴書のRust表現修正完了
- [ ] 面接想定質問回答準備完了

### 理想条件（1ヶ月後）
- [ ] 上記すべて
- [ ] OSSコントリビューション 2-3件
- [ ] 組込みRust小規模プロジェクト完成
- [ ] rust-learning-journey進捗50%以上
- [ ] 技術勉強会参加・発表

---

## 🔗 関連ファイル

- [学習計画](./README.md)
- [GitHub Actions設定](../security_camera_viewer/.github/workflows/ci.yml)
- [ベンチマーク](../security_camera_viewer/benches/performance.rs)
- [LeetCode解答](./leetcode-solutions/)
- [学習ノート](./rust-book-notes/)
- [組込みRust](./embedded-rust/)

---

**作成日**: 2026-03-15
**最終更新**: 2026-03-15

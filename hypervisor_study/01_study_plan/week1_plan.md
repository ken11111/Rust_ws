# Week 1: 仮想化基礎理論

## 🎯 今週の目標
CPU仮想化、メモリ仮想化、I/O仮想化の基本概念を理解し、ハイパーバイザーの理論的基盤を構築する。

## 📚 学習項目

### Day 1-2: CPU仮想化の原理
- [ ] 仮想化可能性（Virtualizability）の概念
- [ ] VMX（Intel VT-x）の基本原理
- [ ] SVM（AMD-V）の基本原理
- [ ] VMCSとVMCBの理解
- [ ] VMExit/VMEntryのメカニズム

**学習リソース**:
- Intel SDM Volume 3, Chapter 23-33
- AMD Architecture Programmer's Manual Volume 2

**成果物**: 02_theory_notes/02_cpu_virtualization/ に詳細ノート

### Day 3-4: メモリ仮想化
- [ ] 仮想メモリとゲスト物理メモリの概念
- [ ] EPT（Extended Page Tables）の仕組み
- [ ] NPT（Nested Page Tables）の仕組み
- [ ] TLBの仮想化と管理
- [ ] メモリアクセス時の変換プロセス

**学習リソース**:
- 「Professional VMware vSphere 5.0」Chapter 2-3
- 論文: "Memory Resource Management in VMware ESX Server"

**成果物**: 02_theory_notes/03_memory_virtualization/ に詳細ノート

### Day 5-6: I/O仮想化
- [ ] I/O仮想化の課題と解決策
- [ ] デバイスエミュレーション vs パススルー
- [ ] IOMMU/VT-dの役割
- [ ] SR-IOVの概念
- [ ] Paravirtualizationアプローチ

**学習リソース**:
- 論文: "I/O Virtualization: A Survey"
- VT-d仕様書

**成果物**: 02_theory_notes/04_io_virtualization/ に詳細ノート

### Day 7: セキュリティモデル
- [ ] ハイパーバイザーのTCB（Trusted Computing Base）
- [ ] Ring -1の概念
- [ ] セキュリティ境界の設計
- [ ] サイドチャネル攻撃とハイパーバイザー

**学習リソース**:
- 論文: "Security and Performance in Cloud Computing"
- NIST SP 800-125

**成果物**: 02_theory_notes/05_security/ に詳細ノート

## 🗓️ 日次スケジュール例

### 平日（2-3時間）
- **19:00-19:30**: 理論学習（論文・ドキュメント読解）
- **19:30-20:30**: 概念整理とノート作成
- **20:30-21:30**: MiniVisorコード概観（対応する部分を探す）
- **21:30-22:00**: その日の学習を05_insights/01_daily_notes/に記録

### 土日（4-6時間）
- **09:00-11:00**: 集中的理論学習
- **11:00-12:00**: 実装確認（MiniVisorコード）
- **14:00-16:00**: 実践課題（小さなコード実験）
- **16:00-17:00**: 週次振り返りと次週計画

## 📝 評価基準

### Week 1終了時にできるべきこと
- [ ] CPU仮想化の基本概念を人に説明できる
- [ ] EPT/NPTの動作原理を図解できる
- [ ] MiniVisorでどの部分が何を担当しているかを特定できる
- [ ] 仮想化技術の歴史と発展を理解している

## 🚀 次週への準備
- Week 2で学ぶハイパーバイザー設計思想の予習
- MiniVisorの全体構造把握の準備
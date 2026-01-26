# Week 1: Type-1仮想化技術の核心（Type-1重点版）

## 🎯 今週の目標
**Type-1ハイパーバイザーの根本原理を完全理解し、ベアメタル仮想化の技術基盤を確実に構築する**

## 📚 Type-1重点学習項目

### Day 1-2: ベアメタル実行とType-1の本質

#### 🔧 技術的核心
- [ ] **ベアメタル実行の原理**
  - ハードウェア電源投入からハイパーバイザー起動まで
  - ファームウェア（UEFI/BIOS）との関係
  - ブートローダー vs ハイパーバイザー直接起動
- [ ] **Type-1の定義と特徴**
  - 「ホストOSが存在しない」ことの技術的意味
  - 物理ハードウェアの直接制御権
  - 最高特権レベルでの実行（Ring -1/EL2）
- [ ] **Type-1 vs Type-2 根本的違い**
  - アーキテクチャレベルでの構造的差異
  - 性能・セキュリティ・拡張性への影響
  - 使用場面の戦略的選択

**学習リソース**:
- Intel SDM Volume 3, Chapter 23 (VMX Introduction)
- "VMware Infrastructure Architecture Overview"
- ARM Architecture Reference Manual, Section D1 (Virtualization)

**成果物**: 02_theory_notes/02_cpu_virtualization/type1_fundamentals.md

### Day 3-4: ハードウェア支援型CPU仮想化（Type-1特化）

#### 🔧 技術的核心
- [ ] **Intel VT-x (VMX) for Type-1**
  - VMXON命令とVMX Root Operation
  - VMCS（Virtual Machine Control Structure）の完全理解
  - VMEntry/VMExit の高速化技術
- [ ] **AMD-V (SVM) for Type-1**
  - VMRUN命令とHost/Guest状態管理
  - VMCB（Virtual Machine Control Block）構造
  - Nested Page Tablesの活用
- [ ] **ARM Virtualization Extensions（MiniVisor重点）**
  - EL2（Exception Level 2）の特権管理
  - HCR_EL2による仮想化制御
  - MiniVisorでの実装確認

**学習リソース**:
- Intel SDM Volume 3C, Chapter 24-33 (VMX詳細)
- AMD Architecture Programmer's Manual Volume 2
- ARM ARM, Section D1.1-D1.6

**成果物**: 02_theory_notes/02_cpu_virtualization/hardware_assisted_type1.md

**実践課題**: MiniVisorのEL2起動確認（src/main.rs:88-90）

### Day 5-6: Type-1メモリ仮想化の深層

#### 🔧 技術的核心
- [ ] **Stage-2/EPT による2段階メモリ変換**
  - Guest Virtual → Guest Physical → Host Physical
  - Type-1でのメモリ分離とセキュリティ
  - パフォーマンス最適化手法
- [ ] **ARM Stage-2 Page Tables（MiniVisor実装）**
  - VTTBR_EL2, VTCR_EL2 設定
  - Stage-2 translation walkthrough
  - Guest Physical Address space設計
- [ ] **Type-1メモリ管理戦略**
  - 物理メモリの分割・割り当て
  - バルーニング、スワッピング
  - NUMA awareness

**学習リソース**:
- Intel SDM Volume 3C, Chapter 28 (EPT)
- ARM ARM, Section D4 (Stage 2 translations)
- "Memory Resource Management in VMware ESX Server"

**成果物**: 02_theory_notes/03_memory_virtualization/type1_memory_architecture.md

**実践課題**: MiniVisorのStage-2設定確認（src/paging.rs）

### Day 7: Type-1セキュリティアーキテクチャ

#### 🔧 技術的核心
- [ ] **Type-1 TCB（Trusted Computing Base）**
  - ハイパーバイザーが唯一の信頼基盤
  - ゲスト間完全分離の実現方法
  - アタックサーフェスの最小化
- [ ] **特権レベル分離（Ring -1/EL2）**
  - ハイパーバイザー vs ゲストカーネル
  - ハードウェア支援セキュリティ機能
  - サイドチャネル攻撃対策
- [ ] **Type-1セキュリティ脅威と対策**
  - ハイパーバイザー脱出攻撃
  - VM間攻撃の防御
  - 物理攻撃からの保護

**学習リソース**:
- "Xen and the Art of Virtualization" (Security aspects)
- Intel TXT/AMD SVM セキュリティ仕様
- "Security in Virtualization" 論文集

**成果物**: 02_theory_notes/05_security/type1_security_model.md

## 🗓️ Type-1重点日次スケジュール

### 平日（2.5-3時間）Type-1集中
- **19:00-19:45**: Type-1理論学習（論文・技術仕様読解）
- **19:45-21:15**: MiniVisorコード分析（Type-1実装確認）
- **21:15-21:45**: 商用Type-1製品調査（VMware/Hyper-V技術文書）
- **21:45-22:00**: Type-1特化の洞察記録

### 土日（4-6時間）Type-1深掘り
- **09:00-11:00**: Type-1理論の集中学習
- **11:00-12:00**: MiniVisor実装詳細確認
- **14:00-16:00**: Type-1プロトタイプ実験
- **16:00-17:00**: Type-1学習成果整理

## 📝 Type-1特化評価基準

### Week 1終了時にできるべきこと
- [ ] Type-1ハイパーバイザーの動作原理を技術者に詳細説明できる
- [ ] ベアメタル実行とハードウェア制御の仕組みを図解できる
- [ ] ARM EL2/Intel VMXによる仮想化メカニズムを理解している
- [ ] MiniVisorがなぜType-1なのかをコードレベルで説明できる
- [ ] 商用Type-1製品（ESXi等）の基本アーキテクチャを理解している

### Type-1理解度チェックポイント
- **基本**: Type-1とType-2の違いを明確に説明できる
- **応用**: Type-1のパフォーマンス優位性を技術的に論証できる
- **実装**: MiniVisorのType-1実装部分を特定・説明できる
- **商用**: VMware vSphereの基本アーキテクチャを理解している

## 🚀 Week 2への準備（Type-1設計深化）

### 次週予習項目
- Type-1設計パターンの分類（マイクロ vs モノリシック）
- MiniVisorの設計選択とトレードオフ
- 商用Type-1製品の差別化技術

### Week 1成果物
- **Type-1基礎理論集**: 完全なType-1技術文書
- **MiniVisor Type-1分析**: ソースコードのType-1実装部分マップ
- **商用製品調査**: VMware/Hyper-V/Xenの基本比較

## 💡 Type-1学習のコツ

### 理論学習のポイント
1. **常にベアメタル実行を意識する**
2. **ハードウェア制御の直接性を理解する**
3. **Type-2との違いを常に対比する**

### 実装確認のポイント
1. **MiniVisorの#![no_std]/#![no_main]の意味**
2. **EL2での起動プロセス**
3. **物理メモリの直接管理**

### 商用製品研究のポイント
1. **Type-1の実世界での優位性**
2. **エンタープライズでのType-1採用理由**
3. **クラウドインフラでのType-1活用**

この集中的なType-1学習により、ハイパーバイザー技術の本質を確実に理解できます。
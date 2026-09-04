# Rust学習記録

## 🎯 目的
超小型人工衛星組込みソフトウェア開発に向けたRust習得

## 📋 学習計画

### Phase 1: 基礎（Week 1-2）⏳進行中
- [ ] The Rust Programming Language 1-10章
  - [ ] Chapter 1-3: 基本構文
  - [ ] Chapter 4: 所有権システム
  - [ ] Chapter 5-6: 構造体とenums
  - [ ] Chapter 7-8: モジュールとコレクション
  - [ ] Chapter 9-10: エラーハンドリングとジェネリクス
- [ ] 所有権・ライフタイムの完全理解
  - 学習ノート: [rust-book-notes/ch04-ownership.md](./rust-book-notes/ch04-ownership.md)
- [ ] LeetCode Easy 20問解答
  - 進捗: 0/20問
  - 証拠: [leetcode-solutions/easy/](./leetcode-solutions/easy/)

### Phase 2: 組込み（Week 3-4）⏳未着手
- [ ] Embedded Rust Book 完読
- [ ] no_std環境の理解
- [ ] LED点滅プログラム実装
  - ターゲット: STM32/ESP32/RP2040
  - コード: [embedded-rust/led-blink/](./embedded-rust/led-blink/)
- [ ] UART通信実装
  - コード: [embedded-rust/uart-echo/](./embedded-rust/uart-echo/)
- [ ] タイマー割り込み実装
  - コード: [embedded-rust/timer-interrupt/](./embedded-rust/timer-interrupt/)

### Phase 3: 実践（Week 5-8）⏳未着手
- [ ] 小規模組込みプロジェクト（センサー読み取り）
- [ ] 非同期処理実装（tokio/async-std）
  - コード: [mini-projects/async-tcp/](./mini-projects/async-tcp/)
- [ ] CLIツール作成
  - コード: [mini-projects/cli-tool/](./mini-projects/cli-tool/)
- [ ] エラーハンドリング実践

## 📊 学習時間記録

| Week | 日付 | 学習時間 | 主な学習内容 |
|------|------|----------|-------------|
| Week 1 | - | 0h | （未開始） |
| **合計** | - | **0h** | - |

## 📚 学習リソース

### 必須
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Embedded Rust Book](https://docs.rust-embedded.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

### 参考
- [LeetCode](https://leetcode.com/)
- [Rustlings](https://github.com/rust-lang/rustlings)
- [Awesome Embedded Rust](https://github.com/rust-embedded/awesome-embedded-rust)

## 🎓 学習メモ

詳細な学習メモは [rust-book-notes/](./rust-book-notes/) 配下に記録

## 📝 今後の予定

- [ ] GitHub Actions CI/CD構築
- [ ] コードカバレッジ測定
- [ ] 技術ブログ執筆（学習内容まとめ）
- [ ] OSSコントリビューション

---

**作成日**: 2026-03-15
**最終更新**: 2026-03-15

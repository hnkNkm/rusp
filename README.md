# Rusp - A Typed Lisp with Rust's Type System

Ruspは、Lispの簡潔なS式構文とRustの強力な型システムを組み合わせた実験的なプログラミング言語です。

## 特徴

- **S式構文**: Lispの伝統的な括弧記法
- **静的型付け**: コンパイル時の型チェック
- **型推論**: 明示的な型注釈が不要な場合は自動推論
- **Rustライクな型システム**: 将来的に所有権・借用の概念を導入予定
- **対話型REPL**: インタラクティブな開発環境

## インストール

```bash
# リポジトリのクローン
git clone https://github.com/yourusername/rusp.git
cd rusp

# ビルド
cargo build --release

# 実行
cargo run
```

## 使い方

REPLを起動:
```bash
$ cargo run
Rusp REPL v0.1.0
Type 'exit' or press Ctrl+C to quit

> 
```

## 現在実装済みの機能

### データ型

| 型 | 説明 | 例 |
|---|------|-----|
| `i32` | 32ビット整数 | `42`, `-10` |
| `f64` | 64ビット浮動小数点 | `3.14`, `-0.5` |
| `bool` | 真偽値 | `true`, `false` |
| `String` | 文字列 | `"hello"`, `"world"` |

### 演算子

#### 算術演算（整数）
- `+` : 加算
- `-` : 減算
- `*` : 乗算
- `/` : 除算

#### 算術演算（浮動小数点）
- `+.` : 加算
- `-.` : 減算
- `*.` : 乗算
- `/.` : 除算

#### 比較演算
- `=` : 等価
- `<` : より小さい
- `>` : より大きい
- `<=` : 以下
- `>=` : 以上

#### 論理演算
- `and` : 論理積
- `or` : 論理和
- `not` : 否定

### 組み込み関数
- `print` : 値を出力
- `println` : 値を出力して改行
- `type-of` : 値の型を返す

## 構文例

### 基本的な計算
```lisp
> (+ 1 2)
3: i32

> (* 3 4)
12: i32

> (+. 1.5 2.5)
4: f64
```

### 変数束縛
```lisp
> (let x 10)
10: i32

> (+ x 5)
15: i32

; 型注釈付き
> (let y i32 42)
42: i32
```

### 条件分岐
```lisp
> (if (> 5 3) "yes" "no")
"yes": String

> (if (and true false) 1 2)
2: i32
```

### 型情報の取得
```lisp
> (type-of 42)
"i32": String

> (type-of "hello")
"String": String
```

## プロジェクト構造

```
src/
├── main.rs         # REPLメインループ
├── ast.rs          # 抽象構文木の定義
├── parser/         # nomベースのパーサー
│   ├── mod.rs      # パーサーのエントリポイント
│   ├── expr.rs     # 式のパース
│   ├── types.rs    # 型注釈のパース
│   └── error.rs    # カスタムエラー型
├── types.rs        # 型チェッカーと型環境
├── eval.rs         # 評価器（インタプリタ）
└── env.rs          # 実行時環境と値の定義
```

### 主要コンポーネント

- **パーサー**: [nom](https://github.com/rust-bakery/nom)パーサーコンビネータライブラリを使用
- **型システム**: 静的型チェックと型推論を実装
- **評価器**: tree-walkingインタプリタ

## エラーハンドリング

```lisp
> (+ 1 "hello")
Error: Type mismatch in argument: expected i32, got String

> 99999999999999999999
Error: 99999999999999999999 is out of i32 range
```

## 今後の実装予定

### Phase 1 (短期)
- [ ] 関数定義 (`defn`)
- [ ] ラムダ式 (`fn`, `lambda`)
- [ ] 再帰関数のサポート

### Phase 2 (中期)
- [ ] リスト型とベクター型
- [ ] パターンマッチング
- [ ] 構造体とレコード型
- [ ] モジュールシステム

### Phase 3 (長期)
- [ ] 所有権システム
- [ ] 借用チェッカー
- [ ] ライフタイム
- [ ] トレイトシステム
- [ ] マクロシステム

## 開発

### ビルド
```bash
cargo build
```

### テスト実行
```bash
cargo test
```

### フォーマット
```bash
cargo fmt
```

### リント
```bash
cargo clippy
```

## ライセンス

[MITライセンス](LICENSE)

## 貢献

プルリクエストを歓迎します！大きな変更の場合は、まずissueを開いて変更内容について議論してください。

## 参考資料

- [言語設計ドキュメント](docs/language-design.md) - Ruspの詳細な仕様
- [Rust公式サイト](https://www.rust-lang.org/)
- [nom パーサーコンビネータ](https://github.com/rust-bakery/nom)
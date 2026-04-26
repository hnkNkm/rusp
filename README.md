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
(blank line cancels a multi-line input)

> 
```

カッコが閉じていない式は自動で複数行入力になります。継続中は `..` プロンプトが出ます。途中で空行を入れると入力をキャンセルできます。

```lisp
> (defn sum [xs: _] -> i32
..   (match xs
..     (nil 0)
..     ((cons h t) (+ h (sum t)))))
> (sum (list 1 2 3 4 5))
15: i32
```

## 現在実装済みの機能

### データ型

| 型 | 説明 | 例 |
|---|------|-----|
| `i32` | 32ビット整数 | `42`, `-10` |
| `i64` | 64ビット整数 | `9223372036854775807` |
| `f64` | 64ビット浮動小数点 | `3.14`, `-0.5` |
| `bool` | 真偽値 | `true`, `false` |
| `String` | 文字列 | `"hello"`, `"world"` |
| `List<T>` | 同種要素のリスト | `(list 1 2 3)`, `nil` |

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

#### 入出力・型
- `print` : 値を出力
- `println` : 値を出力して改行
- `type-of` : 値の型を返す

#### リスト操作
- `cons` : 先頭に要素を追加 `(cons 0 (list 1 2)) → (0 1 2)`
- `car` : 先頭要素を取得
- `cdr` : 先頭を除いた残りのリスト
- `null?` : 空リストか判定
- `length` : 要素数
- `append` : 2つのリストを連結
- `nth` : n番目の要素を取得 (0-indexed)

#### 高階関数
- `map` : `(map f lst)` — 各要素に `f` を適用した新しいリスト
- `filter` : `(filter pred lst)` — 述語 `pred` が真になる要素だけを集めた新しいリスト
- `fold` : `(fold f init lst)` — 左畳み込み (`f : acc -> elem -> acc`)

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

### 関数定義
```lisp
; 引数の型・戻り型を明示
> (defn square [x: i32] -> i32 (* x x))

> (square 7)
49: i32

; 再帰
> (defn fact [n: i32] -> i32
    (if (<= n 1) 1 (* n (fact (- n 1)))))

> (fact 5)
120: i32
```

### ラムダとクロージャ
```lisp
; 匿名関数
> ((fn [x: i32 y: i32] -> i32 (+ x y)) 3 4)
7: i32

; クロージャ (環境をキャプチャ)
> (let adder (fn [x: i32] -> (fn [y: i32] -> i32 (+ x y))))
> ((adder 10) 5)
15: i32
```

### let-in
```lisp
; 局所束縛
> (let x 10 (let y 20 (+ x y)))
30: i32
```

### リスト操作
```lisp
> (list 1 2 3)
(1 2 3): List<i32>

> (cons 0 (list 1 2 3))
(0 1 2 3): List<i32>

> (car (list 1 2 3))
1: i32

> (cdr (list 1 2 3))
(2 3): List<i32>

> (length (list "a" "b" "c"))
3: i32

; 再帰でリスト総和
> (defn sum [lst: List<i32>] -> i32
    (if (null? lst) 0 (+ (car lst) (sum (cdr lst)))))
> (sum (list 1 2 3 4 5))
15: i32
```

### 高階関数
```lisp
; map: 各要素を2乗
> (map (fn [x: i32] -> i32 (* x x)) (list 1 2 3))
(1 4 9): List<i32>

; filter: 正の数だけ残す
> (filter (fn [x: i32] -> bool (> x 0)) (list 1 -2 3 -4))
(1 3): List<i32>

; fold: 畳み込みで総和
> (fold (fn [acc: i32 x: i32] -> i32 (+ acc x)) 0 (list 1 2 3 4 5))
15: i32

; defn で定義した関数も渡せる
> (defn inc [x: i32] -> i32 (+ x 1))
> (map inc (list 10 20 30))
(11 21 31): List<i32>
```

### パターンマッチング
`match` 式でスカラーやリストを構造分解できます。対応パターン:

- リテラル (`1`, `true`, `"foo"` など) — 値が等しいときにマッチ
- `_` — ワイルドカード（何にでもマッチし、束縛しない）
- 変数名 — 何にでもマッチし、その名前で束縛
- `nil` — 空リスト (`nil` または `(list)`) にマッチ
- `(cons head tail)` — 非空リストを先頭と残りに分解（入れ子可）
- `(list p1 p2 ...)` — ちょうどN要素のリストに位置でマッチ（`cons` 連鎖の糖衣）
- `(as <pat> <name>)` — `<pat>` にマッチしつつ、値全体を `<name>` でも束縛
- `(guard <pat> <expr>)` — `<pat>` にマッチしつつ、`<expr>`（bool）が真のときだけ成立。`<pat>` で束縛した変数は `<expr>` 内で使える

`Bool` と `List<T>` を scrutinee にした `match` は **型チェック時に網羅性を検証** します。ケースが漏れていると不足パターンを示すエラーになります（`_` や変数で全受けすれば回避可）:

```
> (match (= 1 1) (true "yes"))
Error: match is not exhaustive: missing patterns: false

> (match (list 1 2) (nil 0))
Error: match is not exhaustive: missing patterns: (cons _ _)
```

ガード付きアーム (`(guard ...)`) は実行時にしか真偽が決まらないため、網羅性判定の根拠にはなりません。

```lisp
> (match 1 (1 "one") (2 "two") (_ "other"))
"one": String

; head/tail への束縛
> (match (list 10 20 30) ((cons h t) h) (nil 0))
10: i32

; 入れ子パターン: 先頭が 1 のリストだけマッチ
> (match (list 1 2 3) ((cons 1 _) "starts-with-one") (_ "other"))
"starts-with-one": String

; リスト総和を再帰 + match で
> (defn sum [xs: _] -> i32
    (match xs
      (nil 0)
      ((cons h t) (+ h (sum t)))))
> (sum (list 1 2 3 4 5))
15: i32

; (list ...) パターン: 固定長のリストを位置で分解
> (match (list 10 20 30) ((list a b c) (+ a (+ b c))) (_ 0))
60: i32

; 要素数が合わないと次のアームへフォールスルー
> (match (list 1 2) ((list a b c) a) (_ 99))
99: i32

; (as ...) パターン: 内側にマッチしつつ全体を別名でも束縛
> (match (list 1 2 3)
    ((as (cons h _) xs) (+ h (length xs)))
    (_ 0))
4: i32

; (guard ...) パターン: 値の条件で絞り込み
> (defn classify [n: i32] -> String
    (match n
      ((guard x (> x 0)) "positive")
      (0                 "zero")
      (x                 "negative")))

> (classify 7)
"positive": String

> (classify -3)
"negative": String
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

### Phase 1 (短期) — 完了
- [x] 関数定義 (`defn`)
- [x] ラムダ式 (`fn`)
- [x] 再帰関数のサポート

### Phase 2 (中期)
- [x] リスト型 (`List<T>`) と基本操作
- [x] 高階関数 (`map`, `filter`, `fold`)
- [x] パターンマッチング (リテラル/変数/ワイルドカード/`nil`/`cons`)
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
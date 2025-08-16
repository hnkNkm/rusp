# Rusp: A Typed Lisp with Rust's Type System

## 概要

Rusp は、Lisp の表現力と Rust の型安全性を融合させた新しいプログラミング言語です。S 式による簡潔な構文と、所有権・借用による安全なメモリ管理を実現します。

## 主要な特徴

- **S 式構文**: Lisp の伝統的な括弧記法
- **静的型付け**: コンパイル時の型チェック
- **所有権システム**: Rust の所有権モデル
- **パターンマッチング**: 強力な分解と条件分岐
- **ゼロコスト抽象化**: 実行時オーバーヘッドなし
- **マクロシステム**: 構文レベルの拡張性

## 基本構文

### 変数定義

```rusp
; 不変変数
(let x i32 42)

; 可変変数
(let-mut y String "hello")

; 型推論
(let z (+ 1 2))  ; z: i32
```

### 関数定義

```rusp
; 基本的な関数
(defn add [a: i32 b: i32] -> i32
  (+ a b))

; ジェネリクス
(defn identity<T> [x: T] -> T
  x)

; 所有権の移動
(defn consume [s: String] -> ()
  (println! s))

; 借用
(defn borrow [s: &String] -> usize
  (len s))

; 可変借用
(defn mutate [s: &mut String] -> ()
  (push-str! s " world"))
```

## 型システム

### プリミティブ型

```rusp
; 整数型
i8, i16, i32, i64, i128, isize
u8, u16, u32, u64, u128, usize

; 浮動小数点
f32, f64

; 文字と文字列
char, String, &str

; 真偽値
bool
```

### 複合型

```rusp
; タプル
(type Point (i32 i32))
(let p Point (10 20))

; 構造体
(struct Person
  [name: String
   age: u32])

; 列挙型
(enum Result<T E>
  (Ok T)
  (Err E))

; トレイトオブジェクト
(type DrawableBox (Box dyn Drawable))
```

## 所有権と借用

```rusp
; 所有権の移動
(let s1 (String::from "hello"))
(let s2 s1)  ; s1は使用不可に

; 借用
(let s (String::from "world"))
(let r &s)   ; 不変借用
(println! r)
(println! s) ; sはまだ使用可能

; 可変借用
(let-mut s (String::from "hello"))
(let r &mut s)
(push-str! r " world")
; sは可変借用が終わるまで使用不可

; ライフタイム注釈
(defn longest<'a> [x: &'a str y: &'a str] -> &'a str
  (if (> (len x) (len y))
    x
    y))
```

## パターンマッチング

```rusp
; 基本的なmatch
(match x
  [0 "zero"]
  [1 "one"]
  [_ "other"])

; 構造体の分解
(match person
  [(Person name: n age: a)
   (format! "{} is {} years old" n a)])

; ガード付き
(match value
  [Some x | (> x 10) "big"]
  [Some x "small"]
  [None "nothing"])

; 列挙型
(match result
  [(Ok value) value]
  [(Err e) (panic! e)])
```

## トレイト

```rusp
; トレイト定義
(trait Display
  (fn fmt [&self] -> String))

; トレイト実装
(impl Display for Person
  (fn fmt [&self] -> String
    (format! "{}, age {}" self.name self.age)))

; トレイト境界
(defn print-it<T: Display> [item: T] -> ()
  (println! (fmt &item)))

; 派生トレイト
#[derive(Debug Clone PartialEq)]
(struct Point [x: f64 y: f64])
```

## マクロシステム

```rusp
; 構文マクロ
(defmacro when [cond & body]
  `(if ~cond
     (do ~@body)
     ()))

; パターンベースマクロ
(defmacro vec! [& args]
  (match args
    [[] `(Vec::new)]
    [[x] `(Vec::from [~x])]
    [[x & xs] `(Vec::from [~x ~@xs])]))

; 手続きマクロ
(proc-macro derive-serialize [item]
  ; 構造体を解析してシリアライズコードを生成
  ...)
```

## エラーハンドリング

```rusp
; Result型
(defn divide [a: f64 b: f64] -> Result<f64 String>
  (if (== b 0.0)
    (Err "Division by zero")
    (Ok (/ a b))))

; ?演算子
(defn calculate [] -> Result<f64 String>
  (let x (divide 10.0 2.0)?)
  (let y (divide x 3.0)?)
  (Ok y))

; Option型
(defn find-user [id: u32] -> Option<User>
  (get users-map id))

; unwrap_or
(let name (unwrap-or (find-user 42) "Unknown"))
```

## 非同期プログラミング

```rusp
; async関数
(async-defn fetch-data [url: String] -> Result<String Error>
  (let response (await (http::get url)))
  (await (response.text)))

; async/await
(async-defn main [] -> ()
  (match (await (fetch-data "https://api.example.com"))
    [(Ok data) (println! data)]
    [(Err e) (eprintln! e)]))

; 並行実行
(let results
  (join-all
    [(fetch-data url1)
     (fetch-data url2)
     (fetch-data url3)]))
```

## メモリ管理

```rusp
; スマートポインタ
(let boxed (Box::new 42))        ; ヒープ割り当て
(let counted (Rc::new "shared"))  ; 参照カウント
(let atomic (Arc::new data))      ; スレッドセーフ参照カウント

; 内部可変性
(let cell (RefCell::new 5))
(borrow-mut! cell (fn [x] (set! x 10)))

; Drop trait
(impl Drop for TempFile
  (fn drop [&mut self] -> ()
    (delete-file self.path)))
```

## 実装例

### FizzBuzz

```rusp
(defn fizzbuzz [n: u32] -> ()
  (for i (range 1 (+ n 1))
    (println!
      (match [(% i 3) (% i 5)]
        [[0 0] "FizzBuzz"]
        [[0 _] "Fizz"]
        [[_ 0] "Buzz"]
        [[_ _] (to-string i)]))))
```

### リスト操作

```rusp
(defn map<T U> [f: Fn(T) -> U lst: Vec<T>] -> Vec<U>
  (let-mut result (Vec::new))
  (for item lst
    (push! result (f item)))
  result)

(defn filter<T> [pred: Fn(&T) -> bool lst: Vec<T>] -> Vec<T>
  (let-mut result (Vec::new))
  (for item lst
    (when (pred &item)
      (push! result item)))
  result)

; 使用例
(let numbers (vec! 1 2 3 4 5))
(let doubled (map (fn [x] (* x 2)) numbers))
(let evens (filter (fn [x] (== (% x 2) 0)) doubled))
```

### 再帰的データ構造

```rusp
; 連結リスト
(enum List<T>
  Nil
  (Cons T (Box List<T>)))

(defn length<T> [lst: &List<T>] -> usize
  (match lst
    [Nil 0]
    [(Cons _ tail) (+ 1 (length tail))]))

; 二分木
(enum Tree<T>
  Leaf
  (Node T (Box Tree<T>) (Box Tree<T>)))

(defn insert<T: Ord> [tree: Tree<T> value: T] -> Tree<T>
  (match tree
    [Leaf (Node value (Box::new Leaf) (Box::new Leaf))]
    [(Node v left right)
     (if (< value v)
       (Node v (Box::new (insert *left value)) right)
       (Node v left (Box::new (insert *right value))))]))
```

## コンパイラ実装のアイデア

### フェーズ

1. **字句解析**: S 式のトークン化
2. **構文解析**: AST の構築
3. **型推論**: Hindley-Milner + 所有権解析
4. **借用チェッカー**: ライフタイム検証
5. **MIR 生成**: 中間表現への変換
6. **最適化**: LLVM IR への変換
7. **コード生成**: ネイティブコード出力

### 特徴的な機能

- **段階的型付け**: 型注釈を省略可能
- **効果システム**: 副作用の追跡
- **コンパイル時計算**: const 関数
- **プラグインシステム**: コンパイラ拡張

## まとめ

Lisp の単純で拡張可能な構文により学習が容易でありながら、Rust の型システムにより実用的で安全なプログラムを書くことができます。

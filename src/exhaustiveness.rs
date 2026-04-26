//! Exhaustiveness checking for `match` expressions.
//!
//! Verifies at type-check time that arms cover every possible value of the
//! scrutinee type, for the structurally-finite types `Bool` and `List<T>`.
//! Other types are considered exhaustive only when at least one arm is
//! irrefutable (wildcard / variable / `(_ as name)`).
//!
//! Algorithm: a width-1 reduction of Maranget's usefulness algorithm.
//! For each constructor of the scrutinee type, recurse into the sub-patterns
//! of arms that match that constructor. Missing constructors become
//! `Witness` values that are rendered back as `Pattern` syntax in the error
//! message — so the user can copy-paste the missing pattern as a new arm.
//!
//! Guarded arms (`Pattern::Guard`) intentionally do NOT contribute to
//! exhaustiveness: their truth value is only known at runtime, matching
//! the Rust/OCaml posture.

use crate::ast::{Pattern, Type};

/// A concrete value not covered by any arm. Rendered into Pattern syntax
/// so the user can drop it directly into the match as a new arm.
enum Witness {
    Wild,
    Bool(bool),
    Nil,
    Cons(Box<Witness>, Box<Witness>),
}

const MAX_DEPTH: usize = 3;

/// Entry point. Returns Ok if `arms` cover every value of `scrutinee`,
/// otherwise an error listing the missing patterns.
pub fn check(scrutinee: &Type, arms: &[&Pattern]) -> Result<(), String> {
    // Flatten top-level `(or p1 p2 ...)` arms into sibling patterns so the
    // existing constructor-based reduction sees them directly. We deliberately
    // do NOT descend into Guard or Cons sub-patterns: guards stay opaque
    // (their truth is runtime-known), and per-constructor recursion handles
    // nested or-patterns through the arm list naturally.
    let mut flat: Vec<&Pattern> = Vec::with_capacity(arms.len());
    for a in arms {
        flatten_or(a, &mut flat);
    }
    let witnesses = missing(scrutinee, &flat, 0);
    if witnesses.is_empty() {
        return Ok(());
    }
    let rendered: Vec<String> = witnesses.iter().map(render).collect();
    Err(format!(
        "match is not exhaustive: missing patterns: {}",
        rendered.join(", ")
    ))
}

/// Flatten top-level `(or ...)` and `(p as name)` wrappers into the
/// underlying constructor patterns. `Guard` is a hard stop — its
/// runtime opacity must be preserved for soundness.
fn flatten_or<'a>(pat: &'a Pattern, out: &mut Vec<&'a Pattern>) {
    match pat {
        Pattern::Or(branches) => {
            for b in branches {
                flatten_or(b, out);
            }
        }
        Pattern::As(inner, _) => flatten_or(inner, out),
        _ => out.push(pat),
    }
}

/// Strip away outer `As` wrappers so structural inspection sees the inner
/// pattern. `(p as name)` has the same coverage as `p`.
fn peel_as(p: &Pattern) -> &Pattern {
    match p {
        Pattern::As(inner, _) => peel_as(inner),
        other => other,
    }
}

/// True iff this pattern matches every value of any type. Guard arms are
/// explicitly excluded — their condition is only known at runtime.
fn arm_is_irrefutable(p: &Pattern) -> bool {
    matches!(peel_as(p), Pattern::Wildcard | Pattern::Variable(_))
}

/// Compute witnesses for values of `ty` not covered by `arms`.
/// `depth` bounds the recursion into nested list types.
fn missing(ty: &Type, arms: &[&Pattern], depth: usize) -> Vec<Witness> {
    // An irrefutable arm covers everything at this level.
    if arms.iter().any(|p| arm_is_irrefutable(p)) {
        return Vec::new();
    }

    match ty {
        Type::Bool => {
            let mut out = Vec::new();
            // Deterministic order: false before true.
            for v in [false, true] {
                if !bool_covered(v, arms) {
                    out.push(Witness::Bool(v));
                }
            }
            out
        }
        Type::List(elem) => {
            let mut out = Vec::new();
            // Deterministic order: nil before cons.
            if !nil_covered(arms) {
                out.push(Witness::Nil);
            }
            if let Some(cons_witness) = missing_cons(elem, arms, depth) {
                out.push(cons_witness);
            }
            out
        }
        // Inferred: skip silently. Useful exhaustiveness needs concrete
        // types, which arrive with bidirectional inference (#8).
        Type::Inferred => Vec::new(),
        // Infinite / opaque types: only irrefutable arms can cover them,
        // and the early-return above already handled that case.
        _ => vec![Witness::Wild],
    }
}

fn bool_covered(v: bool, arms: &[&Pattern]) -> bool {
    arms.iter().any(|p| match peel_as(p) {
        Pattern::LiteralBool(b) => *b == v,
        // Guard / literal mismatches / structural patterns don't cover.
        _ => false,
    })
}

fn nil_covered(arms: &[&Pattern]) -> bool {
    arms.iter().any(|p| matches!(peel_as(p), Pattern::Nil))
}

/// Returns Some(witness) when there exists a cons value not covered by any
/// arm, None when every cons value is covered. Recurses into head/tail
/// patterns so nested structure (e.g. `List<Bool>`) is handled precisely.
fn missing_cons(elem: &Type, arms: &[&Pattern], depth: usize) -> Option<Witness> {
    // Collect (head, tail) pairs from cons-shaped arms. A wildcard or
    // variable arm at this level was already handled by the irrefutable
    // early-return in `missing`, so we don't see it here.
    let mut head_pats: Vec<&Pattern> = Vec::new();
    let mut tail_pats: Vec<&Pattern> = Vec::new();
    let mut any_cons = false;
    for p in arms {
        if let Pattern::Cons(h, t) = peel_as(p) {
            any_cons = true;
            head_pats.push(h);
            tail_pats.push(t);
        }
    }

    if !any_cons {
        // No cons arm at all → the entire cons constructor is missing.
        return Some(Witness::Cons(
            Box::new(Witness::Wild),
            Box::new(Witness::Wild),
        ));
    }

    // Beyond MAX_DEPTH, stop expanding — render tails as `_` to keep
    // error messages readable.
    if depth >= MAX_DEPTH {
        return None;
    }

    // Recurse into the head (element type) and tail (same list type).
    let head_witnesses = missing(elem, &head_pats, depth + 1);
    let list_ty = Type::List(Box::new(elem.clone()));
    let tail_witnesses = missing(&list_ty, &tail_pats, depth + 1);

    // If both head and tail are fully covered, the cons case is exhaustive.
    if head_witnesses.is_empty() && tail_witnesses.is_empty() {
        return None;
    }

    // Pick the first head/tail witness as a representative. Showing a
    // single concrete missing case is more actionable than enumerating
    // the cross product.
    let head = head_witnesses.into_iter().next().unwrap_or(Witness::Wild);
    let tail = tail_witnesses.into_iter().next().unwrap_or(Witness::Wild);
    Some(Witness::Cons(Box::new(head), Box::new(tail)))
}

fn render(w: &Witness) -> String {
    match w {
        Witness::Wild => "_".to_string(),
        Witness::Bool(b) => b.to_string(),
        Witness::Nil => "nil".to_string(),
        Witness::Cons(h, t) => format!("(cons {} {})", render(h), render(t)),
    }
}

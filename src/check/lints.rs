//! Warnings: unused variables, unreachable patterns, ranges that can be
//! narrowed or merged, dead code, code after something that never finishes,
//! loop labels nothing uses, handling an `Err` that can't be one,
//! doc comments that document nothing, and types with no values since each
//! would hold another without end.
//! None of these make a program wrong, so none of them stop it. The naming
//! lints are in `naming.rs`, since their fixes rename, which checks the
//! renamed source, and `unused_mut` is the borrow checker's, which knows
//! what's changed. `#[allow(..)]` turns any of them off (`allow.rs`).

use super::scope::FnId;
use super::{Checker, Local, Scoped, adt_names};
use crate::ast::*;
use crate::error::{Edit, Fix, Lint, Span, Warning};
use crate::exhaust;
use crate::types::all_fields;
use std::collections::HashSet;

/// A variable nothing reads. The fix renames it to start with `_`, which says
/// that's on purpose, everywhere it's bound.
pub(super) fn unused(local: &Local) -> Warning {
    let name = &local.name;
    let edits = local
        .sites
        .iter()
        .map(|site| match site.shorthand {
            true => Edit::insert(site.span.end, format!(": _{name}")),
            false => Edit::insert(site.span.start, "_"),
        })
        .collect();
    Warning {
        lint: Lint::UnusedVariables,
        span: local.sites[0].span,
        message: format!("unused variable `{name}`"),
        fixes: vec![Fix::new(format!("rename to `_{name}`"), edits)],
    }
}

/// A call of one of `Result`'s methods for an `Err`, `.method(..)` at
/// `span`, from the method's name to the end of the call.
#[derive(Clone)]
pub(super) struct Unwrap {
    pub method: String,
    pub span: Span,
}

impl Unwrap {
    /// What's for getting what's in an `Ok`, and doing something else for
    /// an `Err`. Not `map_err` or `or_else`, which can make a `Result<T, !>`
    /// into one with an error type another `?` wants.
    pub const METHODS: [&str; 4] = ["unwrap", "expect", "unwrap_or", "unwrap_or_else"];
}

/// `call`, of a `Result` whose error type is `!`, which can't be an `Err`:
/// what it does for one never happens. For `unwrap` and `expect`, that's a
/// panic, which its caller must still say it may have; for `unwrap_or` and
/// `unwrap_or_else`, a default that's made, or a function that's passed,
/// for nothing. The fix is `into_ok()`, which can't panic.
///
/// Only where the error type is `!` as the function is checked, not where
/// it's a type parameter that's `!` in some uses: there what's done for an
/// `Err` is needed by the others.
pub(super) fn unwrap_infallible(call: &Unwrap) -> Warning {
    let m = &call.method;
    let why = match m.as_str() {
        "unwrap" | "expect" => "it says it may panic, though it can't".to_string(),
        "unwrap_or" => "the default is made for nothing".to_string(),
        _ => "the function is never called".to_string(),
    };
    Warning {
        lint: Lint::UnwrapInfallible,
        span: call.span,
        message: format!(
            "`{m}` of a `Result` whose error type is `!`, which can't be an `Err`: {why}"
        ),
        fixes: vec![Fix::new(
            "use `into_ok()`",
            vec![Edit {
                span: call.span,
                text: "into_ok()".into(),
            }],
        )],
    }
}

/// A statement `e;` at `span`, whose value is a `Result` that may be an
/// `Err`: the error is thrown away with it, unseen. A `Result` is
/// `Discard` when its values are, so nothing else stops that; this is
/// Rust's `must_use` on `Result`, for one type. The fix says it's on
/// purpose, `let _ = e;`, which does the same. Not when the error type is
/// `!`: then there's no error to lose.
pub(super) fn unused_results(span: Span) -> Warning {
    Warning {
        lint: Lint::UnusedResults,
        span,
        message: "this `Result` is thrown away, and any error in it unseen: handle it, pass it on \
                  with `?`, or write `let _ =` to throw it away"
            .into(),
        fixes: vec![Fix::new(
            "throw it away with `let _ =`",
            vec![Edit::insert(span.start, "let _ = ")],
        )],
    }
}

/// The statement or expression at `span` never runs: what comes before it
/// never finishes.
pub(super) fn unreachable_code(span: Span) -> Warning {
    Warning {
        lint: Lint::UnreachableCode,
        span,
        message: "unreachable code: what comes before it never finishes, so this never runs".into(),
        fixes: Vec::new(),
    }
}

/// A name a `use` brings in that nothing uses. The fix removes the `use`,
/// if that's all it brings in.
pub(super) fn unused_import(import: &super::modules::Import) -> Warning {
    let name = &import.name.name;
    Warning {
        lint: Lint::UnusedImports,
        span: import.name.span,
        message: format!("unused import `{name}`"),
        fixes: import
            .alone
            .map(|decl| Fix::new("remove the `use`", vec![Edit::delete(decl)]))
            .into_iter()
            .collect(),
    }
}

/// A loop's label that no `break` or `continue` uses. The fix removes it.
pub(super) fn unused_label(label: &Label) -> Warning {
    let name = &label.name.name;
    Warning {
        lint: Lint::UnusedLabels,
        span: label.name.span,
        message: format!("unused label `{name}`: no `break` or `continue` names it"),
        fixes: vec![Fix::new(
            format!("remove `{name}:`"),
            vec![Edit::delete(label.head)],
        )],
    }
}

/// Arm `i` can never match. The fix deletes it, along with the `,` and the
/// whitespace that separate it from its neighbours.
pub(super) fn unreachable_arm(arms: &[Arm], i: usize) -> Warning {
    let span = arms[i].pat.span;
    let delete = if let Some(next) = arms.get(i + 1) {
        Some(Span::new(span.start, next.pat.span.start))
    } else if i > 0 {
        // The last arm: from the end of the one before, which leaves any
        // trailing `,` behind for that one.
        Some(Span::new(arms[i - 1].body.span.end, arms[i].body.span.end))
    } else {
        // The only arm, and the `,` after it can't be found from here.
        None
    };
    Warning {
        lint: Lint::UnreachablePatterns,
        span,
        message: "unreachable pattern: the arms above already match everything it would".into(),
        fixes: delete
            .map(|d| Fix::new("remove the unreachable arm", vec![Edit::delete(d)]))
            .into_iter()
            .collect(),
    }
}

/// The or-pattern alternative at `span`, somewhere in `pats`, can never
/// match. The fix deletes it and the `|` next to it.
pub(super) fn unreachable_alt(span: Span, pats: &[Pattern]) -> Warning {
    let fix = pats
        .iter()
        .find_map(|p| alternatives(p, span))
        .map(|(alts, k)| {
            let delete = match k {
                0 => Span::new(alts[0].span.start, alts[1].span.start),
                _ => Span::new(alts[k - 1].span.end, alts[k].span.end),
            };
            Fix::new(
                "remove the unreachable alternative",
                vec![Edit::delete(delete)],
            )
        });
    Warning {
        lint: Lint::UnreachablePatterns,
        span,
        message:
            "unreachable alternative: the patterns before it already match everything it would"
                .into(),
        fixes: fix.into_iter().collect(),
    }
}

/// The alternatives of the or-pattern in `p` that has one at `span`, and
/// which one that is.
fn alternatives(p: &Pattern, span: Span) -> Option<(&[Pattern], usize)> {
    match &p.kind {
        PatternKind::Or(alts) => alts
            .iter()
            .position(|a| a.span == span)
            .map(|k| (alts.as_slice(), k))
            .or_else(|| alts.iter().find_map(|a| alternatives(a, span))),
        PatternKind::Variant(_, _, _, pats) => pats.iter().find_map(|p| alternatives(p, span)),
        PatternKind::Struct(_, _, fields, _) => {
            fields.iter().find_map(|f| alternatives(&f.pat, span))
        }
        _ => None,
    }
}

/// The range at `span`, somewhere in `pats`, only reaches `lo..=hi`.
pub(super) fn narrowable(
    span: Span,
    lo: Option<i128>,
    hi: Option<i128>,
    pats: &[Pattern],
) -> Warning {
    let (inclusive, lit) = pats
        .iter()
        .find_map(|p| written_range(p, span))
        .unwrap_or((true, Lit::Int));
    let range = exhaust::show_range(lo, hi, inclusive, lit);
    Warning {
        lint: Lint::OverlappingRanges,
        span,
        message: format!(
            "this range can be narrowed to `{range}`: the patterns before it already match the rest"
        ),
        fixes: vec![Fix::new(
            format!("narrow to `{range}`"),
            vec![Edit::replace(span, range)],
        )],
    }
}

/// Alternatives of the or-pattern `p` that touch end to end, like
/// `0..10 | 10..20`, each run as a warning naming the one range that covers
/// it. At least one of them must be a range: `1 | 2` is fine as it is.
///
/// The fix writes the merged range in place of whichever of them comes first
/// in the source, and deletes the others with the `|` before each.
pub(super) fn mergeable(p: &Pattern, alts: &[Pattern]) -> Vec<Warning> {
    // `(lo, hi, is a range, written with ..=, index in alts)`
    // An open end is past every value: an integer is at most `u64::MAX`.
    let (min, max) = (i128::MIN, i128::MAX);
    let mut ints: Vec<(i128, i128, bool, bool, usize)> = Vec::new();
    let mut lit = Lit::Int;
    for (k, a) in alts.iter().enumerate() {
        match a.kind {
            PatternKind::Int(n, l) => {
                ints.push((n, n, false, true, k));
                lit = l;
            }
            PatternKind::Range(lo, hi, inclusive, l) => {
                ints.push((lo.unwrap_or(min), hi.unwrap_or(max), true, inclusive, k));
                lit = l;
            }
            _ => {}
        }
    }
    ints.sort_by_key(|&(lo, ..)| lo);
    let open = |lo: i128, hi: i128| ((lo != min).then_some(lo), (hi != max).then_some(hi));
    ints.chunk_by(|a, b| a.1.checked_add(1) == Some(b.0))
        .filter(|run| run.len() > 1 && run.iter().any(|&(_, _, range, _, _)| range))
        .map(|run| {
            let names: Vec<String> = run
                .iter()
                .map(|&(lo, hi, _, inclusive, _)| {
                    let (lo, hi) = open(lo, hi);
                    format!("`{}`", exhaust::show_range(lo, hi, inclusive, lit))
                })
                .collect();
            let (last, rest) = names.split_last().expect("a run has two or more");
            // Written in the style of the highest one.
            let &(_, hi, _, inclusive, _) = run.last().expect("a run has two or more");
            let (lo, hi) = open(run[0].0, hi);
            let merged = exhaust::show_range(lo, hi, inclusive, lit);

            let mut ks: Vec<usize> = run.iter().map(|&(.., k)| k).collect();
            ks.sort_unstable();
            let mut edits = vec![Edit::replace(alts[ks[0]].span, merged.clone())];
            // Every `k` here is past `ks[0]`, so it has a `|` before it.
            edits.extend(
                ks[1..]
                    .iter()
                    .map(|&k| Edit::delete(Span::new(alts[k - 1].span.end, alts[k].span.end))),
            );
            Warning {
                lint: Lint::MergeableRanges,
                span: p.span,
                message: format!(
                    "{} and {last} can be merged into `{merged}`",
                    rest.join(", ")
                ),
                fixes: vec![Fix::new(format!("merge into `{merged}`"), edits)],
            }
        })
        .collect()
}

/// Whether the range pattern at `span` inside `p` was written with `..=`,
/// and what its ends are written as.
fn written_range(p: &Pattern, span: Span) -> Option<(bool, Lit)> {
    match &p.kind {
        PatternKind::Range(_, _, inclusive, lit) if p.span == span => Some((*inclusive, *lit)),
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) => {
            pats.iter().find_map(|p| written_range(p, span))
        }
        PatternKind::Struct(_, _, fields, _) => {
            fields.iter().find_map(|f| written_range(&f.pat, span))
        }
        _ => None,
    }
}

/// A doc comment before something that can't have one, like a statement,
/// or before nothing at all. The fix makes it a plain comment, which says
/// that's what was meant, by taking a `/` off each line.
pub(super) fn unused_doc(doc: &DocComment) -> Warning {
    let edits = doc
        .lines
        .iter()
        .map(|l| Edit::delete(Span::new(l.span.start + 2, l.span.start + 3)))
        .collect();
    Warning {
        lint: Lint::UnusedDocComments,
        span: doc.span(),
        message: match doc.inner {
            true => "unused doc comment: a `//!` one documents the module it's at the start of, \
                     so it goes before anything else in the module"
                .into(),
            false => "unused doc comment: only a `fn`, `struct`, `enum`, field or variant can \
                      have one"
                .into(),
        },
        fixes: vec![Fix::new("make it a plain `//` comment", edits)],
    }
}

/// What one function refers to, for finding dead code.
#[derive(Clone, Default)]
pub(super) struct Uses {
    /// Functions it names, called or not.
    pub(super) fns: HashSet<FnId>,
    /// Structs and enums its signature or body mentions, by symbol.
    pub(super) types: HashSet<String>,
    /// Structs it constructs.
    pub(super) structs: HashSet<String>,
    /// Enum variants it constructs, as `(enum's symbol, variant)`.
    pub(super) variants: HashSet<(String, String)>,
}

fn dead(span: Span, message: String) -> Warning {
    Warning {
        lint: Lint::DeadCode,
        span,
        message,
        fixes: Vec::new(),
    }
}

impl Checker {
    /// Warns about code that can never run: functions `main` can't reach,
    /// and structs, enums and variants that no such function constructs.
    /// Their names say what's dead; one starting with `_` says that's on
    /// purpose. A nested function can only be reached from its block, so
    /// one that nothing there uses is dead too. A nested function or local
    /// type in a dead function goes without saying, so that's all that's
    /// reported.
    ///
    /// There's no fix: removing one dead function can break another that
    /// calls it.
    ///
    /// What's exported is live: other packages may use it.
    ///
    /// The methods of impls of traits, and the defaults of traits, are
    /// taken to be live: which ones a call of a trait's method runs depends
    /// on types, which may be decided far away. An inherent method is dead
    /// if nothing live calls it.
    pub(super) fn dead_code(&mut self, prog: &Program) {
        // Everything reachable from `main`, by following which function
        // names which. Functions that only call each other aren't.
        let fns = prog.all_fns();
        let main = fns.iter().position(|(f, parent, method)| {
            parent.is_none() && method.is_none() && f.module == ROOT && f.name.name == "main"
        });
        let mut live = vec![false; self.fns.len()];
        let mut todo = vec![FnId(main.expect("checked that there's a `main`"))];
        todo.extend(self.kept_measures.iter().copied());
        for (i, (f, parent, method)) in fns.iter().enumerate() {
            let inherent =
                matches!(method, Some(MethodOf::Impl(k)) if prog.impls[*k].trait_name.is_none());
            // What's exported is other packages' to use.
            let exported = f.vis.is_export() && parent.is_none();
            if (method.is_some() && !inherent) || exported {
                todo.push(FnId(i));
            }
        }
        while let Some(f) = todo.pop() {
            if !std::mem::replace(&mut live[f.0], true) {
                todo.extend(self.uses[f.0].fns.iter().copied());
            }
        }

        let mut used = Uses::default();
        for (f, uses) in self.uses.iter().zip(&live) {
            if *uses {
                used.types.extend(f.types.iter().cloned());
                used.structs.extend(f.structs.iter().cloned());
                used.variants.extend(f.variants.iter().cloned());
            }
        }
        // A type mentioned by a live one is live too.
        let mut todo: Vec<String> = used.types.iter().cloned().collect();
        while let Some(t) = todo.pop() {
            let mut inner = HashSet::new();
            // Its fields as declared, mentioning its own type parameters.
            let generic = self.adts[&t].generic(&t);
            for ft in all_fields(&self.adts, &generic) {
                adt_names(&ft, &mut inner);
            }
            for name in inner {
                if used.types.insert(name.clone()) {
                    todo.push(name);
                }
            }
        }

        let on_purpose = |id: &Ident| id.name.starts_with('_');
        // The libraries' are their own business.
        let own = self.fns.iter().zip(&live).zip(&fns).skip(self.from.all_fns);
        for ((f, is_live), (.., method)) in own {
            let in_live = f.parent.is_none_or(|p| live[p.0]);
            if !is_live && in_live && !on_purpose(&f.name) {
                let what = if method.is_some() {
                    "method"
                } else {
                    "function"
                };
                let message = format!("{what} `{}` is never used", f.name.name);
                self.warnings.push(dead(f.name.span, message));
            }
        }
        // An alias is gone once resolved, so it's used if it's named at all.
        let named: HashSet<&String> = (self.type_names.values())
            .filter_map(|s| match s {
                Scoped::Adt(symbol) if self.aliases.contains_key(symbol) => Some(symbol),
                _ => None,
            })
            .collect();
        let types = prog.all_types().into_iter().zip(&self.types);
        for ((d, _), def) in types.skip(self.from.all_types) {
            let in_live = def.parent.is_none_or(|p| live[p.0]);
            if on_purpose(d.name()) || !in_live || d.vis().is_export() {
                continue;
            }
            let symbol = &def.symbol;
            match d {
                TypeDecl::Struct(s) => {
                    if !used.structs.contains(symbol) {
                        let message = format!("struct `{}` is never constructed", s.name.name);
                        self.warnings.push(dead(s.name.span, message));
                    }
                }
                TypeDecl::Alias(a) => {
                    if !named.contains(symbol) {
                        let message = format!("type alias `{}` is never used", a.name.name);
                        self.warnings.push(dead(a.name.span, message));
                    }
                }
                TypeDecl::Enum(e) if !used.types.contains(symbol) => {
                    let message = format!("enum `{}` is never used", e.name.name);
                    self.warnings.push(dead(e.name.span, message));
                }
                TypeDecl::Enum(e) => {
                    for v in &e.variants {
                        let key = (symbol.clone(), v.name.name.clone());
                        if !on_purpose(&v.name) && !used.variants.contains(&key) {
                            let message = format!(
                                "variant `{}::{}` is never constructed",
                                e.name.name, v.name.name
                            );
                            self.warnings.push(dead(v.name.span, message));
                        }
                    }
                }
            }
        }
    }
}

impl Checker {
    /// Warns about structs and enums that hold themselves with no way to
    /// stop, so that they have no values: `struct Stream { next:
    /// Box<Stream> }`, or `A` and `B` that each hold the other. Each is
    /// likely missing a variant that doesn't, like a list's `Nil`. A type
    /// with no values for another reason, like `enum Never {}` or one that
    /// holds a `Never`, is fine, and so is one that only holds one of these,
    /// which the warning on that covers.
    ///
    /// Such a type can't be constructed, so `dead_code` says so too: this
    /// says why instead.
    pub(super) fn infinite_types(&mut self, prog: &Program) {
        let types = prog.all_types().into_iter().zip(&self.types);
        for ((d, _), def) in types.skip(self.from.all_types) {
            if let TypeDecl::Alias(_) = d {
                continue;
            }
            let symbol = &def.symbol;
            let generic = self.adts[symbol].generic(symbol);
            if d.name().name.starts_with('_')
                || !exhaust::endless(&self.adts, &generic)
                || !holds_itself(&self.adts, symbol)
            {
                continue;
            }
            let (kind, name, variants) = match d {
                TypeDecl::Struct(s) => ("struct", &s.name, &[][..]),
                TypeDecl::Enum(e) => ("enum", &e.name, &e.variants[..]),
                TypeDecl::Alias(_) => unreachable!("skipped above"),
            };
            let spans: Vec<Span> = std::iter::once(name.span)
                .chain(variants.iter().map(|v| v.name.span))
                .collect();
            self.warnings
                .retain(|w| !(w.lint == Lint::DeadCode && spans.contains(&w.span)));
            self.warnings.push(Warning {
                lint: Lint::InfiniteTypes,
                span: name.span,
                message: format!(
                    "{kind} `{n}` has no values: each would hold another `{n}`, without end; it \
                     needs a way to stop, like a variant that doesn't hold one",
                    n = name.name
                ),
                fixes: Vec::new(),
            });
        }
    }
}

/// Does the type `symbol` hold itself, through its fields' types?
fn holds_itself(adts: &crate::types::Adts, symbol: &str) -> bool {
    let mut seen = HashSet::new();
    let mut todo = vec![symbol.to_string()];
    while let Some(t) = todo.pop() {
        let mut inner = HashSet::new();
        for ft in all_fields(adts, &adts[&t].generic(&t)) {
            adt_names(&ft, &mut inner);
        }
        for name in inner {
            if name == symbol {
                return true;
            }
            if seen.insert(name.clone()) {
                todo.push(name);
            }
        }
    }
    false
}

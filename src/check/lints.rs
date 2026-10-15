//! Warnings: unused variables, unreachable patterns, ranges that can be
//! narrowed or merged, dead code, and doc comments that document nothing.
//! None of these make a program wrong, so none of them stop it.

use super::scope::FnId;
use super::{Checker, Local, adt_names};
use crate::ast::*;
use crate::error::{Edit, Fix, Lint, Span, Warning};
use crate::exhaust;
use crate::types::{Type, all_fields};
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
pub(super) fn narrowable(span: Span, lo: i64, hi: i64, pats: &[Pattern]) -> Warning {
    let inclusive = pats.iter().find_map(|p| written_inclusive(p, span));
    let range = exhaust::show_range(lo, hi, inclusive.unwrap_or(true));
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
    let mut ints: Vec<(i64, i64, bool, bool, usize)> = alts
        .iter()
        .enumerate()
        .filter_map(|(k, a)| match a.kind {
            PatternKind::Int(n) => Some((n, n, false, true, k)),
            PatternKind::Range(lo, hi, inclusive) => Some((lo, hi, true, inclusive, k)),
            _ => None,
        })
        .collect();
    ints.sort_by_key(|&(lo, ..)| lo);
    ints.chunk_by(|a, b| a.1.checked_add(1) == Some(b.0))
        .filter(|run| run.len() > 1 && run.iter().any(|&(_, _, range, _, _)| range))
        .map(|run| {
            let names: Vec<String> = run
                .iter()
                .map(|&(lo, hi, _, inclusive, _)| {
                    format!("`{}`", exhaust::show_range(lo, hi, inclusive))
                })
                .collect();
            let (last, rest) = names.split_last().expect("a run has two or more");
            // Written in the style of the highest one.
            let &(_, hi, _, inclusive, _) = run.last().expect("a run has two or more");
            let merged = exhaust::show_range(run[0].0, hi, inclusive);

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

/// Whether the range pattern at `span` inside `p` was written with `..=`.
fn written_inclusive(p: &Pattern, span: Span) -> Option<bool> {
    match &p.kind {
        PatternKind::Range(_, _, inclusive) if p.span == span => Some(*inclusive),
        PatternKind::Variant(_, _, _, pats) | PatternKind::Or(pats) => {
            pats.iter().find_map(|p| written_inclusive(p, span))
        }
        PatternKind::Struct(_, _, fields, _) => {
            fields.iter().find_map(|f| written_inclusive(&f.pat, span))
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
        message: "unused doc comment: only a `fn`, `struct`, `enum`, field or variant can have one"
            .into(),
        fixes: vec![Fix::new("make it a plain `//` comment", edits)],
    }
}

/// What one function refers to, for finding dead code.
#[derive(Default)]
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
    pub(super) fn dead_code(&mut self, prog: &Program) {
        // Everything reachable from `main`, by following which function
        // names which. Functions that only call each other aren't. The
        // top-level ones come first, so `main`'s id is where it is in
        // `prog.fns`.
        let main = prog.fns.iter().position(|f| f.name.name == "main");
        let mut live = vec![false; self.fns.len()];
        let mut todo = vec![FnId(main.expect("checked that there's a `main`"))];
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
            let params = self.adts[&t].params.iter().map(|p| Type::Param(p.clone()));
            for ft in all_fields(&self.adts, &Type::Adt(t, params.collect())) {
                adt_names(&ft, &mut inner);
            }
            for name in inner {
                if used.types.insert(name.clone()) {
                    todo.push(name);
                }
            }
        }

        let on_purpose = |id: &Ident| id.name.starts_with('_');
        for (f, is_live) in self.fns.iter().zip(&live) {
            let in_live = f.parent.is_none_or(|p| live[p.0]);
            if !is_live && in_live && !on_purpose(&f.name) {
                let message = format!("function `{}` is never used", f.name.name);
                self.warnings.push(dead(f.name.span, message));
            }
        }
        for ((d, _), def) in prog.all_types().into_iter().zip(&self.types) {
            let in_live = def.parent.is_none_or(|p| live[p.0]);
            if on_purpose(d.name()) || !in_live {
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

//! The naming lints, as in Rust: `UpperCamelCase` for structs, enums,
//! variants, traits and type parameters (`non_camel_case_types`), and
//! `snake_case` for modules, functions, methods, parameters, variables and
//! fields (`non_snake_case`).
//! Underscores before or after a name don't count, and digits are fine
//! (`Vec2`, `x1`). An acronym is one word, so `HTTPServer` should be
//! `HttpServer`.
//!
//! Each name gets one warning, where it's declared. The fix renames it
//! everywhere it's used (see `rename.rs`), so it's only there when that
//! works: the program checks, and the new name is a name, and doesn't
//! clash with anything.

use crate::ast::*;
use crate::check::Info;
use crate::error::{Fix, Lint, Span, Warning};
use crate::source::Sources;

/// Which way a name should be written.
#[derive(Clone, Copy)]
enum Case {
    Camel,
    Snake,
    Upper,
}

/// A name declared in the program, with what it is, in words.
struct Decl<'a> {
    name: &'a str,
    span: Span,
    what: &'static str,
    case: Case,
}

/// A warning for each name in `prog` that's written the wrong way. `info`
/// is what checking `sources` found, if it checked, for the fixes.
pub fn warnings(sources: &Sources, prog: &Program, info: Option<&Info>) -> Vec<Warning> {
    decls(prog)
        .into_iter()
        .filter_map(|d| {
            let want = match d.case {
                Case::Camel => camel(d.name),
                Case::Snake => snake(d.name),
                Case::Upper => snake(d.name).to_uppercase(),
            };
            if want == d.name {
                return None;
            }
            let fix = info.and_then(|info| {
                let edits = crate::rename::rename(sources, prog, info, d.span, &want).ok()?;
                Some(Fix::new(format!("rename to `{want}`"), edits))
            });
            let (lint, case) = match d.case {
                Case::Camel => (Lint::NonCamelCaseTypes, "an upper camel case"),
                Case::Snake => (Lint::NonSnakeCase, "a snake case"),
                Case::Upper => (Lint::NonUpperCaseGlobals, "an upper case"),
            };
            // Unless it's a keyword, say, it's worth suggesting even if it
            // clashes with something.
            let suggestion = match crate::rename::is_name(&want) {
                true => format!(": `{want}`"),
                false => String::new(),
            };
            Some(Warning {
                lint,
                span: d.span,
                message: format!(
                    "{} `{}` should have {case} name{suggestion}",
                    d.what, d.name
                ),
                fixes: fix.into_iter().collect(),
            })
        })
        .collect()
}

/// `name` in `UpperCamelCase`: `MyPoint` for `my_point`, `HttpServer` for
/// `HTTPServer`. Underscores around it stay.
pub fn camel(name: &str) -> String {
    let (before, words, after) = words(name);
    let mut out = before.to_string();
    let mut last: Option<char> = None;
    for w in words {
        let first = w.chars().next().expect("words aren't empty");
        // Two words that case can't tell apart, like `1` and `2`, keep the
        // `_` between them.
        if last.is_some_and(|l| !has_case(l) && !has_case(first)) {
            out.push('_');
        }
        out.extend(first.to_uppercase());
        out.push_str(&w[first.len_utf8()..].to_lowercase());
        last = w.chars().last();
    }
    out + after
}

/// `name` in `snake_case`: `my_fn` for `myFn`, `http_server` for
/// `HTTPServer`. Underscores around it stay.
pub fn snake(name: &str) -> String {
    let (before, words, after) = words(name);
    let words: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
    format!("{before}{}{after}", words.join("_"))
}

fn has_case(c: char) -> bool {
    c.is_lowercase() || c.is_uppercase()
}

/// The underscores before `name`, its words, and the underscores after.
/// Words are split at `_`, and where the case changes: before an upper
/// case letter after a lower case one or a digit (`my|Fn`, `vec2|D`), and
/// before the last of a run of upper case letters if a lower case one
/// follows (`HTTP|Server`).
fn words(name: &str) -> (&str, Vec<&str>, &str) {
    let inner = name.trim_matches('_');
    if inner.is_empty() {
        return (name, Vec::new(), "");
    }
    let start = name.len() - name.trim_start_matches('_').len();
    let end = start + inner.len();
    let mut words = Vec::new();
    for part in inner.split('_').filter(|p| !p.is_empty()) {
        let chars: Vec<(usize, char)> = part.char_indices().collect();
        let mut from = 0;
        for (k, &(i, c)) in chars.iter().enumerate().skip(1) {
            let prev = chars[k - 1].1;
            let next = chars.get(k + 1).map(|&(_, c)| c);
            let split = c.is_uppercase()
                && (prev.is_lowercase()
                    || prev.is_ascii_digit()
                    || (prev.is_uppercase() && next.is_some_and(char::is_lowercase)));
            if split {
                words.push(&part[from..i]);
                from = i;
            }
        }
        words.push(&part[from..]);
    }
    (&name[..start], words, &name[end..])
}

/// Every name `prog` declares, in order: of types, type parameters, fields
/// and variants, of functions and their parameters, and of the variables
/// in them. A variable bound in every alternative of an or-pattern is
/// declared in the first.
fn decls(prog: &Program) -> Vec<Decl<'_>> {
    let mut out: Vec<Decl> = Vec::new();
    for m in &prog.mods[1..] {
        out.push(ident(&m.name, "module", Case::Snake));
    }
    for (d, _) in prog.all_types() {
        out.push(ident(d.name(), "type", Case::Camel));
        params(d.generics(), &mut out);
        match d {
            TypeDecl::Struct(s) => {
                out.extend(
                    s.fields
                        .iter()
                        .map(|f| ident(&f.name, "field", Case::Snake)),
                );
            }
            TypeDecl::Enum(e) => {
                out.extend(
                    e.variants
                        .iter()
                        .map(|v| ident(&v.name, "variant", Case::Camel)),
                );
            }
        }
    }
    for t in &prog.traits {
        // Named by its symbol once resolved: `shapes::Show` in a module.
        let name = t.name.name.rsplit("::").next().unwrap_or_default();
        out.push(Decl {
            name,
            ..ident(&t.name, "trait", Case::Camel)
        });
        // An impl's are named by the trait's.
        out.extend(
            t.assoc
                .iter()
                .map(|a| ident(&a.name, "associated type", Case::Camel)),
        );
        out.extend(
            t.effects
                .iter()
                .map(|a| ident(&a.name, "associated effect", Case::Snake)),
        );
        // Those with a default are among the functions.
        for m in t.methods.iter().filter(|m| !m.provided) {
            out.push(ident(&m.decl.name, "method", Case::Snake));
            params(&m.decl.generics, &mut out);
            let ps = m.decl.params.iter().filter(|p| p.name.name != "self");
            out.extend(ps.map(|p| ident(&p.name, "variable", Case::Snake)));
        }
    }
    for d in &prog.impls {
        params(&d.generics, &mut out);
    }
    for (f, _, method) in prog.all_fns() {
        match method {
            // Named by the trait: the warning is there.
            Some(MethodOf::Impl(k)) if prog.impls[k].trait_name.is_some() => {}
            Some(_) => out.push(ident(&f.name, "method", Case::Snake)),
            None => out.push(ident(&f.name, "function", Case::Snake)),
        }
        params(&f.generics, &mut out);
        out.extend(
            f.params
                .iter()
                .map(|p| ident(&p.name, "variable", Case::Snake)),
        );
        variables(&f.body, &mut out);
    }
    out.sort_by_key(|d| d.span.start);
    out
}

fn ident<'a>(id: &'a Ident, what: &'static str, case: Case) -> Decl<'a> {
    Decl {
        name: &id.name,
        span: id.span,
        what,
        case,
    }
}

fn params<'a>(g: &'a Generics, out: &mut Vec<Decl<'a>>) {
    out.extend(
        g.types
            .iter()
            .map(|p| match (g.is_effect(p), g.is_const(p)) {
                (true, _) => ident(p, "effect parameter", Case::Snake),
                // A constant, as a global would be.
                (_, true) => ident(p, "const parameter", Case::Upper),
                _ => ident(p, "type parameter", Case::Camel),
            }),
    );
}

/// Adds the variables `e` binds, in `let`s, closures and `match` arms, to
/// `out`. Not those of the functions nested in it.
fn variables<'a>(e: &'a Expr, out: &mut Vec<Decl<'a>>) {
    match &e.kind {
        ExprKind::Block(stmts, _) => {
            for stmt in stmts {
                if let Stmt::Let { pat, .. } = stmt {
                    bindings(pat, out);
                }
            }
        }
        ExprKind::Match(_, arms) => arms.iter().for_each(|a| bindings(&a.pat, out)),
        ExprKind::Lambda(l) => {
            out.extend(
                l.params
                    .iter()
                    .map(|p| ident(&p.name, "variable", Case::Snake)),
            );
        }
        _ => {}
    }
    for c in e.children() {
        variables(c, out);
    }
}

fn bindings<'a>(p: &'a Pattern, out: &mut Vec<Decl<'a>>) {
    match &p.kind {
        PatternKind::Move(_) => unreachable!("written by lift"),
        PatternKind::Bind(name, _) => out.push(Decl {
            name,
            span: p.name_span(),
            what: "variable",
            case: Case::Snake,
        }),
        // Each alternative binds the same variables.
        PatternKind::Or(alts) => alts.iter().take(1).for_each(|p| bindings(p, out)),
        PatternKind::Variant(_, _, _, pats) | PatternKind::Array(pats) => {
            pats.iter().for_each(|p| bindings(p, out))
        }
        PatternKind::Struct(_, _, fields, _) => fields.iter().for_each(|f| bindings(&f.pat, out)),
        PatternKind::Wild
        | PatternKind::Int(..)
        | PatternKind::Range(..)
        | PatternKind::Bool(_)
        | PatternKind::Unit => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_like_rustc() {
        assert_eq!(snake("myFn"), "my_fn");
        assert_eq!(snake("HTTPServer"), "http_server");
        assert_eq!(snake("x1"), "x1");
        assert_eq!(snake("vec2D"), "vec2_d");
        assert_eq!(snake("a__b"), "a_b");
        assert_eq!(snake("_myVar"), "_my_var");
        assert_eq!(snake("type_"), "type_");
        assert_eq!(snake("_"), "_");
        assert_eq!(camel("my_point"), "MyPoint");
        assert_eq!(camel("HTTPServer"), "HttpServer");
        assert_eq!(camel("Vec2"), "Vec2");
        assert_eq!(camel("IO"), "Io");
        assert_eq!(camel("t"), "T");
        assert_eq!(camel("T"), "T");
        assert_eq!(camel("_unused"), "_Unused");
        assert_eq!(camel("v_1_2"), "V1_2");
        assert_eq!(camel("myPoint"), "MyPoint");
    }
}

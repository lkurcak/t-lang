//! `#[allow(..)]` and `#[warn(..)]`: turning lints off and on. One on a
//! `fn`, `struct`, `enum`, field or variant says so of the lints it names
//! in all of it: a `fn`'s parameters and body, the functions and types
//! nested in it, a type's fields and variants. One on a `mod` says so in
//! the module, the files of `mod m;` in it included, and so does
//! `#![allow(..)]` or `#![warn(..)]` at the top of the module: of the crate
//! root, in all the program. Where several say something of a lint, the
//! innermost decides, and of those on the same thing, the last. Where none
//! does, a lint is on unless it's one of those off by default
//! (`Lint::on_by_default`). As in Rust, a name that isn't a lint's is only
//! a warning (`unknown_lints`), so that a program keeps working when a lint
//! is renamed or removed.
//!
//! Lints only ever add warnings, so turning one on or off never changes
//! what a program means.

use crate::ast::*;
use crate::error::{Edit, Fix, Lint, Span, Warning};
use crate::source::Sources;

/// What an attribute says of one lint in a part of the program.
struct Scope {
    span: Span,
    /// How many modules out from where it's written the part is, for the
    /// files of modules: those of a module and of the modules it's in
    /// cover the same file.
    depth: usize,
    /// Its place among all of them, in the order they're written.
    order: usize,
    lint: Lint,
    warn: bool,
}

/// Adds a warning for each name in `prog`'s `#[allow(..)]`s and
/// `#[warn(..)]`s that isn't a lint, then drops the warnings of lints
/// that are off where they are.
pub fn apply(prog: &Program, sources: &Sources, warnings: &mut Vec<Warning>) {
    let mut scopes: Vec<Scope> = Vec::new();
    for (span, depth, allows) in allowed(prog, sources) {
        for a in allows {
            for name in &a.lints {
                match Lint::from_name(&name.name) {
                    Some(lint) => scopes.push(Scope {
                        span,
                        depth,
                        order: scopes.len(),
                        lint,
                        warn: a.warn,
                    }),
                    None => warnings.push(unknown(name)),
                }
            }
        }
    }
    warnings.retain(|w| {
        let innermost = (scopes.iter())
            .filter(|s| {
                s.lint == w.lint && s.span.start <= w.span.start && w.span.end <= s.span.end
            })
            .min_by_key(|s| {
                (
                    s.span.end - s.span.start,
                    s.depth,
                    std::cmp::Reverse(s.order),
                )
            });
        match innermost {
            Some(s) => s.warn,
            None => w.lint.on_by_default(),
        }
    });
    warnings.sort_by_key(|w| w.span.start);
}

/// Everything in `prog` that has attributes, as the part of the program
/// they're of, how many modules out that is from where they're written,
/// and the attributes: a module that has files is in them too, and the
/// crate root's own are of all of it.
fn allowed<'p>(prog: &'p Program, sources: &Sources) -> Vec<(Span, usize, &'p [Allow])> {
    let mut out: Vec<(Span, usize, &[Allow])> = Vec::new();
    out.push((
        Span::new(0, usize::MAX),
        usize::MAX,
        &prog.mods[ROOT].inner_allows,
    ));
    for m in &prog.mods[1..] {
        out.push((m.span, 0, &m.allows));
        out.push((m.span, 0, &m.inner_allows));
    }
    let first = sources.first().start;
    for file in sources.files().iter().filter(|f| f.start != first) {
        let mut m = Some(file.module);
        let mut depth = 0;
        while let Some(i) = m.filter(|&i| i != ROOT) {
            out.push((file.span(), depth, &prog.mods[i].allows));
            out.push((file.span(), depth, &prog.mods[i].inner_allows));
            m = prog.mods[i].parent;
            depth += 1;
        }
    }
    for (f, ..) in prog.all_fns() {
        out.push((f.span, 0, &f.allows));
    }
    for (d, _) in prog.all_types() {
        out.push((d.span(), 0, d.allows()));
        match d {
            TypeDecl::Struct(s) => {
                out.extend(
                    s.fields
                        .iter()
                        .map(|f| (f.name.span.to(f.ty.span), 0, &f.allows[..])),
                );
            }
            TypeDecl::Enum(e) => out.extend(e.variants.iter().map(|v| (v.span, 0, &v.allows[..]))),
            TypeDecl::Alias(_) => {}
        }
    }
    out.retain(|(_, _, allows)| !allows.is_empty());
    out
}

fn unknown(name: &Ident) -> Warning {
    let close = Lint::ALL
        .into_iter()
        .map(|l| (distance(&name.name, l.name()), l.name()))
        .min()
        .filter(|&(d, _)| d <= 2 + name.name.len() / 4);
    let fixes = close
        .map(|(_, l)| Fix::new(format!("use `{l}`"), vec![Edit::replace(name.span, l)]))
        .into_iter()
        .collect();
    Warning {
        notes: Vec::new(),
        lint: Lint::UnknownLints,
        span: name.span,
        message: format!("unknown lint `{}`", name.name),
        fixes,
    }
}

/// How many characters to add, remove or change to turn `a` into `b`.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diag = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let next = (diag + usize::from(ca != cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            diag = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_edits() {
        assert_eq!(distance("dead_code", "dead_code"), 0);
        assert_eq!(distance("dead_cod", "dead_code"), 1);
        assert_eq!(distance("unused_variable", "unused_variables"), 1);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "abc"), 3);
    }
}

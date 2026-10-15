//! `#[allow(..)]`: turning lints off. One on a `fn`, `struct`, `enum`,
//! field or variant turns off the lints it names in all of it: a `fn`'s
//! parameters and body, the functions and types nested in it, a type's
//! fields and variants. As in Rust, a name that isn't a lint's is only a
//! warning (`unknown_lints`), so that a program keeps working when a lint is
//! renamed or removed.

use crate::ast::*;
use crate::error::{Edit, Fix, Lint, Span, Warning};

/// Adds a warning for each name in `prog`'s `#[allow(..)]`s that isn't a
/// lint, then drops the warnings they turn off.
pub fn apply(prog: &Program, warnings: &mut Vec<Warning>) {
    let mut scopes: Vec<(Span, Lint)> = Vec::new();
    for (span, allows) in allowed(prog) {
        for name in allows.iter().flat_map(|a| &a.lints) {
            match Lint::from_name(&name.name) {
                Some(lint) => scopes.push((span, lint)),
                None => warnings.push(unknown(name)),
            }
        }
    }
    warnings.retain(|w| {
        !scopes
            .iter()
            .any(|&(s, lint)| lint == w.lint && s.start <= w.span.start && w.span.end <= s.end)
    });
    warnings.sort_by_key(|w| w.span.start);
}

/// Everything in `prog` that has attributes, as its span and them.
fn allowed(prog: &Program) -> Vec<(Span, &[Allow])> {
    let mut out: Vec<(Span, &[Allow])> = Vec::new();
    for (f, _) in prog.all_fns() {
        out.push((f.span, &f.allows));
    }
    for (d, _) in prog.all_types() {
        out.push((d.span(), d.allows()));
        match d {
            TypeDecl::Struct(s) => {
                out.extend(
                    s.fields
                        .iter()
                        .map(|f| (f.name.span.to(f.ty.span), &f.allows[..])),
                );
            }
            TypeDecl::Enum(e) => out.extend(e.variants.iter().map(|v| (v.span, &v.allows[..]))),
        }
    }
    out.retain(|(_, allows)| !allows.is_empty());
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

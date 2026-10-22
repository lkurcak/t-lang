//! The `missing_safety_comments` lint: an `unsafe` block says that the
//! obligations of the unsafe operations in it are kept, and a `// SAFETY:`
//! comment just before it says why, for whoever reads it next (see
//! docs/09-unsafe.md). Just before is on the lines of comments right above
//! the line the block starts on.

use crate::ast::*;
use crate::error::{Lint, Span, Warning};
use crate::source::Sources;

pub fn warnings(sources: &Sources, prog: &Program) -> Vec<Warning> {
    let mut blocks = Vec::new();
    for (f, _) in prog.all_fns() {
        unsafe_blocks(&f.body, &mut blocks);
    }
    let text = sources.text();
    blocks
        .into_iter()
        .filter(|&at| !sources.in_library(at) && !said_why(text, at))
        .map(|at| Warning {
            lint: Lint::MissingSafetyComments,
            span: Span::new(at, at + "unsafe".len()),
            message: "this `unsafe` block has no `// SAFETY:` comment saying why its \
                      obligations are kept"
                .into(),
            fixes: Vec::new(),
        })
        .collect()
}

/// Adds where each `unsafe` block in `e` starts to `out`.
fn unsafe_blocks(e: &Expr, out: &mut Vec<usize>) {
    if let ExprKind::Unsafe(_) = e.kind {
        out.push(e.span.start);
    }
    for c in e.children() {
        unsafe_blocks(c, out);
    }
}

/// Is there a `// SAFETY:` comment among the lines of comments just above
/// the line of `text` that `at` is on?
fn said_why(text: &str, at: usize) -> bool {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    text[..line_start]
        .lines()
        .rev()
        .map(str::trim)
        .take_while(|l| l.starts_with("//"))
        .any(|l| {
            l.trim_start_matches('/')
                .trim_start()
                .starts_with("SAFETY:")
        })
}

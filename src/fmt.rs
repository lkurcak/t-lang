//! `t fmt`: prints a program in the one T style, keeping its comments and
//! single blank lines. There are no options. See docs/fmt.md.

use crate::ast::{DocComment, Program};
use crate::error::Error;
use crate::lexer::{self, Comment, Tok, Token};
use crate::parser;
use crate::trivia::Trivia;

#[derive(Debug)]
pub enum FmtError {
    /// The program doesn't parse, so there's nothing to format.
    Syntax(Error),
    /// The formatted program isn't the same program, or lost a comment. A
    /// bug in `t fmt`; the file is left alone.
    Changed(String),
}

/// Formats a whole file. Fails rather than return anything that parses to
/// a different program, or has different comments.
pub fn format(src: &str) -> Result<String, FmtError> {
    let (toks, comments) = lexer::lex_with_comments(src).map_err(FmtError::Syntax)?;
    let program = parser::parse(toks.clone()).map_err(FmtError::Syntax)?;
    let texts = comment_texts(src, &comments);
    let trivia = trivia(src, toks, comments, &program);
    let out = crate::pretty::format(&program, trivia);

    let changed = |why: &str| FmtError::Changed(format!("{why}; this is a bug in `t fmt`"));
    let (toks, comments) =
        lexer::lex_with_comments(&out).map_err(|_| changed("the output doesn't lex"))?;
    let reparsed = parser::parse(toks).map_err(|_| changed("the output doesn't parse"))?;
    if shape(&reparsed) != shape(&program) {
        return Err(changed("the output is a different program"));
    }
    if comment_texts(&out, &comments) != texts {
        return Err(changed("the output has different comments"));
    }
    Ok(out)
}

/// The comments and blank lines to keep. The printer prints doc comments
/// from the AST, but a doc comment that documents nothing isn't in a place
/// the printer knows to put it, so it's kept like a plain comment.
fn trivia<'s>(
    src: &'s str,
    toks: Vec<Token>,
    mut comments: Vec<Comment>,
    program: &Program,
) -> Trivia<'s> {
    let toks = toks
        .into_iter()
        .filter(|t| !matches!(t.tok, Tok::Doc(_)))
        .collect();
    let stray = program.stray_docs.iter().flat_map(|d| &d.lines);
    comments.extend(stray.map(|l| Comment { span: l.span }));
    comments.sort_by_key(|c| c.span.start);
    Trivia::new(src, toks, comments)
}

fn comment_texts<'s>(src: &'s str, comments: &[Comment]) -> Vec<&'s str> {
    comments
        .iter()
        .map(|c| &src[c.span.start..c.span.end])
        .collect()
}

/// The program with its spans and node ids left out, to compare two
/// programs by. `Debug` is derived for every AST type, so this shows every
/// other field. A span prints as `Span { start: 1, end: 2 }` and an id as
/// `NodeId(3)`, which nothing else can: names print in quotes.
///
/// Doc comments that document nothing are compared line by line: which
/// lines they are matters, not how they're grouped into runs, which moving
/// them to the end of a struct can change.
pub fn shape(program: &Program) -> String {
    let mut program = program.clone();
    program.stray_docs = std::mem::take(&mut program.stray_docs)
        .into_iter()
        .flat_map(|d| d.lines)
        .map(|l| DocComment { lines: vec![l] })
        .collect();
    program.next_id = 0;
    let debug = format!("{program:?}");
    let debug = strip(&debug, "Span { ", '}');
    strip(&debug, "NodeId(", ')')
}

/// `debug` without each run of it from `open` to the next `close`.
fn strip(debug: &str, open: &str, close: char) -> String {
    let mut out = String::with_capacity(debug.len());
    let mut rest = debug;
    while let Some(i) = rest.find(open) {
        out.push_str(&rest[..i]);
        let end = rest[i..].find(close).expect("it ends") + i;
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

impl FmtError {
    pub fn render(&self, path: &str, src: &str) -> String {
        match self {
            FmtError::Syntax(e) => e.render(path, src),
            FmtError::Changed(why) => format!("error: not formatting `{path}`: {why}"),
        }
    }
}

/// A unified diff from `old` to `new`, line by line, with three lines of
/// context, as `diff -u` would print it.
pub fn diff(path: &str, old: &str, new: &str) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let ops = diff_ops(&a, &b);

    // Each op with the line numbers it's at in `a` and `b`.
    let mut at = Vec::with_capacity(ops.len());
    let (mut i, mut j) = (0, 0);
    for &op in &ops {
        at.push((op, i, j));
        match op {
            Op::Same => (i, j) = (i + 1, j + 1),
            Op::Del => i += 1,
            Op::Add => j += 1,
        }
    }

    const CONTEXT: usize = 3;
    let mut out = format!("--- {path}\n+++ {path} (formatted)\n");
    let changes: Vec<usize> = (0..at.len()).filter(|&k| at[k].0 != Op::Same).collect();
    let mut k = 0;
    while k < changes.len() {
        // Grow a hunk while the next change is close enough to share context.
        let first = changes[k];
        let mut last = first;
        while k + 1 < changes.len() && changes[k + 1] - last <= 2 * CONTEXT + 1 {
            k += 1;
            last = changes[k];
        }
        k += 1;
        let lo = first.saturating_sub(CONTEXT);
        let hi = (last + CONTEXT + 1).min(at.len());
        let (a_start, b_start) = (at[lo].1, at[lo].2);
        let a_len = at[lo..hi].iter().filter(|x| x.0 != Op::Add).count();
        let b_len = at[lo..hi].iter().filter(|x| x.0 != Op::Del).count();
        let range = |start: usize, len: usize| match len {
            0 => format!("{start},0"),
            1 => format!("{}", start + 1),
            _ => format!("{},{len}", start + 1),
        };
        out.push_str(&format!(
            "@@ -{} +{} @@\n",
            range(a_start, a_len),
            range(b_start, b_len)
        ));
        for &(op, i, j) in &at[lo..hi] {
            match op {
                Op::Same => out.push_str(&format!(" {}\n", a[i])),
                Op::Del => out.push_str(&format!("-{}\n", a[i])),
                Op::Add => out.push_str(&format!("+{}\n", b[j])),
            }
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Op {
    Same,
    Del,
    Add,
}

/// The edits that turn `a` into `b`: a longest common subsequence, after
/// taking off the lines both start and end with.
fn diff_ops(a: &[&str], b: &[&str]) -> Vec<Op> {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (am, bm) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let mut ops = vec![Op::Same; prefix];
    let (n, m) = (am.len(), bm.len());
    if n * m > 1 << 24 {
        // Too big to be worth it: replace the middle wholesale.
        ops.extend(std::iter::repeat_n(Op::Del, n));
        ops.extend(std::iter::repeat_n(Op::Add, m));
    } else {
        // lcs[i][j]: the longest common subsequence of am[i..] and bm[j..].
        let mut lcs = vec![vec![0u32; m + 1]; n + 1];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i][j] = if am[i] == bm[j] {
                    lcs[i + 1][j + 1] + 1
                } else {
                    lcs[i + 1][j].max(lcs[i][j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && am[i] == bm[j] {
                ops.push(Op::Same);
                (i, j) = (i + 1, j + 1);
            } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
                ops.push(Op::Del);
                i += 1;
            } else {
                ops.push(Op::Add);
                j += 1;
            }
        }
    }
    ops.extend(std::iter::repeat_n(Op::Same, suffix));
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_shows_changed_lines_with_context() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";
        let new = "a\nb\nc\nd\nE\nf\ng\nh\ni\nj\nk\n";
        assert_eq!(
            diff("x.t", old, new),
            "--- x.t\n+++ x.t (formatted)\n\
             @@ -2,9 +2,10 @@\n b\n c\n d\n-e\n+E\n f\n g\n h\n i\n j\n+k\n"
        );
        assert_eq!(diff("x.t", old, old), "--- x.t\n+++ x.t (formatted)\n");
    }

    #[test]
    fn shape_ignores_spans() {
        let parse = |src| parser::parse(lexer::lex(src).unwrap()).unwrap();
        let a = parse("fn main() { f(1, 2) }");
        let b = parse("fn main() {\n    f(\n        1,\n        2,\n    )\n}\n");
        let c = parse("fn main() { f(1, 3) }");
        assert_eq!(shape(&a), shape(&b));
        assert_ne!(shape(&a), shape(&c));
    }
}

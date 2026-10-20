//! A layout engine after Wadler's "A prettier printer": the printer
//! describes a program as a `Doc`, with places it may break a line, and
//! `render` picks where to break so that lines fit in `WIDTH` columns.
//!
//! The key part is a `Group`. It prints *flat*, with each `Line` in it as a
//! space, if all of it fits on the rest of the line; otherwise it *breaks*,
//! and each `Line` directly in it starts a new line. Groups nest, and each
//! decides for itself, outermost first, so an outer group can break while
//! the groups inside it stay flat. This is the algorithm Prettier uses: it
//! looks only at the rest of the current line to decide, so it runs in
//! linear time, unlike Wadler's lazy original that keeps alternatives open.

use std::ops::Add;

/// The longest a line should be. A line can still be longer if there's no
/// place to break it, or because of a comment at its end.
pub const WIDTH: usize = 100;

const INDENT: &str = "    ";

#[derive(Debug, Clone)]
pub enum Doc {
    /// Text with no newlines in it.
    Text(String),
    Concat(Vec<Doc>),
    /// Lines in it start one level further in.
    Nest(Box<Doc>),
    /// Prints flat if it fits, and otherwise breaks. `hard` is set if there's
    /// a `HardLine` or `Comment` somewhere inside, so it can never be flat.
    Group {
        doc: Box<Doc>,
        hard: bool,
    },
    /// A space if the group it's in is flat, or else a new line.
    Line,
    /// Nothing if the group it's in is flat, or else a new line.
    SoftLine,
    /// Always a new line.
    HardLine,
    /// Nothing, but the groups around it break: after a comment on a line
    /// of its own, which the next thing mustn't follow on that line.
    BreakParent,
    /// The first if the group it's in breaks, the second if it's flat.
    IfBreak(Box<Doc>, Box<Doc>),
    /// A comment that goes at the end of the line, after everything else on
    /// it. It doesn't count toward the width, so whether there's a comment
    /// doesn't change how the code around it is laid out.
    Comment(String),
    /// The arguments of a call whose last argument is a closure, block,
    /// `match` or struct literal: `normal`, unless that doesn't fit on one
    /// line, and `prefix` (the other arguments, flat) and the start of
    /// `last` do. Then the arguments stay on the call's line and the last
    /// one breaks inside:
    ///
    /// ```text
    /// apply(x, |y| {
    ///     ..
    /// })
    /// ```
    Hug {
        normal: Box<Doc>,
        prefix: Box<Doc>,
        last: Box<Doc>,
        close: Box<Doc>,
    },
}

pub fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

pub fn nest(d: Doc) -> Doc {
    Doc::Nest(Box::new(d))
}

pub fn group(d: Doc) -> Doc {
    let hard = d.hard();
    Doc::Group {
        doc: Box::new(d),
        hard,
    }
}

pub fn if_break(broken: impl Into<Doc>, flat: impl Into<Doc>) -> Doc {
    Doc::IfBreak(Box::new(broken.into()), Box::new(flat.into()))
}

pub fn concat(docs: impl IntoIterator<Item = Doc>) -> Doc {
    docs.into_iter().fold(Doc::nil(), |a, b| a + b)
}

impl Doc {
    pub fn nil() -> Doc {
        Doc::Concat(Vec::new())
    }

    /// Does this force the groups around it to break?
    pub fn hard(&self) -> bool {
        match self {
            Doc::HardLine | Doc::BreakParent | Doc::Comment(_) => true,
            Doc::Group { hard, .. } => *hard,
            Doc::Concat(docs) => docs.iter().any(Doc::hard),
            Doc::Nest(d) => d.hard(),
            // A hard line in `broken` only shows if the group breaks anyway.
            Doc::IfBreak(_, flat) => flat.hard(),
            Doc::Hug { normal, .. } => normal.hard(),
            Doc::Text(_) | Doc::Line | Doc::SoftLine => false,
        }
    }
}

impl From<&str> for Doc {
    fn from(s: &str) -> Doc {
        text(s)
    }
}

impl From<&String> for Doc {
    fn from(s: &String) -> Doc {
        text(s.as_str())
    }
}

impl From<String> for Doc {
    fn from(s: String) -> Doc {
        text(s)
    }
}

impl<D: Into<Doc>> Add<D> for Doc {
    type Output = Doc;

    fn add(self, other: D) -> Doc {
        let other = other.into();
        match (self, other) {
            (Doc::Concat(mut a), Doc::Concat(b)) => {
                a.extend(b);
                Doc::Concat(a)
            }
            (Doc::Concat(mut a), b) => {
                a.push(b);
                Doc::Concat(a)
            }
            (a, b) => Doc::Concat(vec![a, b]),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Flat,
    Break,
}

/// One line of output: the code, and the comment at its end, if any.
struct OutLine {
    code: String,
    comment: Option<String>,
}

/// Lays `doc` out in `WIDTH` columns, and lines up the comments at the ends
/// of lines (see `align_comments`).
pub fn render(doc: &Doc) -> String {
    let mut lines = Vec::new();
    let mut code = String::new();
    let mut comment: Option<String> = None;
    let mut column = 0;
    // What's left to print, last first: indentation, mode, doc.
    let mut stack: Vec<(usize, Mode, &Doc)> = vec![(0, Mode::Break, doc)];
    while let Some((indent, mode, doc)) = stack.pop() {
        match doc {
            Doc::Text(s) => {
                code.push_str(s);
                column += s.chars().count();
            }
            Doc::Concat(docs) => stack.extend(docs.iter().rev().map(|d| (indent, mode, d))),
            Doc::Nest(d) => stack.push((indent + 1, mode, d)),
            Doc::Group { doc, hard } => {
                let mode = match mode {
                    Mode::Flat => Mode::Flat,
                    Mode::Break if !hard && fits(WIDTH - column.min(WIDTH), doc, &stack) => {
                        Mode::Flat
                    }
                    Mode::Break => Mode::Break,
                };
                stack.push((indent, mode, doc));
            }
            Doc::Hug {
                normal,
                prefix,
                last,
                close,
            } => {
                let width = WIDTH - column.min(WIDTH);
                let hug = mode == Mode::Break
                    && (normal.hard() || !fits(width, normal, &stack))
                    && !prefix.hard()
                    && flat_width(prefix).is_some_and(|w| first_line_fits(width, w, last));
                if hug {
                    stack.push((indent, Mode::Break, close));
                    stack.push((indent, Mode::Break, last));
                    stack.push((indent, Mode::Flat, prefix));
                } else {
                    stack.push((indent, mode, normal));
                }
            }
            Doc::IfBreak(broken, flat) => {
                stack.push((
                    indent,
                    mode,
                    if mode == Mode::Break { broken } else { flat },
                ));
            }
            Doc::BreakParent => {}
            Doc::Comment(c) => {
                comment = Some(match comment.take() {
                    Some(before) => format!("{before} {c}"),
                    None => c.clone(),
                });
            }
            Doc::Line | Doc::SoftLine if mode == Mode::Flat => {
                if matches!(doc, Doc::Line) {
                    code.push(' ');
                    column += 1;
                }
            }
            Doc::Line | Doc::SoftLine | Doc::HardLine => {
                lines.push(OutLine {
                    code: std::mem::take(&mut code),
                    comment: comment.take(),
                });
                code = INDENT.repeat(indent);
                column = indent * INDENT.len();
            }
        }
    }
    lines.push(OutLine { code, comment });
    align_comments(&mut lines);

    let mut out = String::new();
    for line in &lines {
        out.push_str(line.code.trim_end());
        out.push('\n');
    }
    // One newline at the end of the file, whatever the doc ended with.
    let end = out.trim_end().len();
    out.truncate(end);
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Would `doc` fit in `width` columns, flat, followed by `rest` up to its
/// first line break?
fn fits(width: usize, doc: &Doc, rest: &[(usize, Mode, &Doc)]) -> bool {
    let mut width = width as isize;
    let mut todo: Vec<(Mode, &Doc)> = vec![(Mode::Flat, doc)];
    let mut rest = rest.iter().rev();
    loop {
        let Some((mode, doc)) = todo.pop().or_else(|| rest.next().map(|&(_, m, d)| (m, d))) else {
            return true;
        };
        match doc {
            Doc::Text(s) => width -= s.chars().count() as isize,
            Doc::Concat(docs) => todo.extend(docs.iter().rev().map(|d| (mode, d))),
            Doc::Nest(d) => todo.push((mode, d)),
            Doc::Group { doc, hard } => {
                todo.push((if *hard { Mode::Break } else { mode }, doc));
            }
            Doc::Hug { normal, .. } => todo.push((mode, normal)),
            Doc::IfBreak(broken, flat) => {
                todo.push((mode, if mode == Mode::Break { broken } else { flat }));
            }
            Doc::Comment(_) | Doc::BreakParent => {}
            Doc::Line if mode == Mode::Flat => width -= 1,
            Doc::SoftLine if mode == Mode::Flat => {}
            Doc::Line | Doc::SoftLine | Doc::HardLine => return true,
        }
        if width < 0 {
            return false;
        }
    }
}

/// `doc` as it prints flat, which can't break, if it can go on one line.
pub fn flat(doc: &Doc) -> Option<Doc> {
    Some(match doc {
        Doc::Text(_) => doc.clone(),
        Doc::Concat(docs) => concat(docs.iter().map(flat).collect::<Option<Vec<_>>>()?),
        Doc::Nest(d) | Doc::Group { doc: d, .. } => flat(d)?,
        Doc::Hug { normal, .. } => flat(normal)?,
        Doc::IfBreak(_, d) => flat(d)?,
        Doc::Line => text(" "),
        Doc::SoftLine => Doc::nil(),
        Doc::HardLine | Doc::BreakParent | Doc::Comment(_) => return None,
    })
}

/// How wide `doc` is on one line, if it can go on one.
pub fn flat_width(doc: &Doc) -> Option<usize> {
    let mut width = 0;
    let mut todo = vec![doc];
    while let Some(doc) = todo.pop() {
        match doc {
            Doc::Text(s) => width += s.chars().count(),
            Doc::Concat(docs) => todo.extend(docs.iter().rev()),
            Doc::Nest(d) | Doc::Group { doc: d, .. } => todo.push(d),
            Doc::Hug { normal, .. } => todo.push(normal),
            Doc::IfBreak(_, flat) => todo.push(flat),
            Doc::Line => width += 1,
            Doc::SoftLine => {}
            Doc::HardLine | Doc::BreakParent | Doc::Comment(_) => return None,
        }
    }
    Some(width)
}

/// Does `doc`, broken, fit in `width` columns up to its first line break,
/// after `used` columns? Groups in it are taken as broken too, so this is
/// how much has to go on the first line at the least.
fn first_line_fits(width: usize, used: usize, doc: &Doc) -> bool {
    let mut width = width as isize - used as isize;
    let mut todo = vec![doc];
    while let Some(doc) = todo.pop() {
        match doc {
            Doc::Text(s) => width -= s.chars().count() as isize,
            Doc::Concat(docs) => todo.extend(docs.iter().rev()),
            Doc::Nest(d) | Doc::Group { doc: d, .. } => todo.push(d),
            Doc::Hug { normal, .. } => todo.push(normal),
            Doc::IfBreak(broken, _) => todo.push(broken),
            Doc::Comment(_) | Doc::BreakParent => {}
            Doc::Line | Doc::SoftLine | Doc::HardLine => return width >= 0,
        }
        if width < 0 {
            return false;
        }
    }
    true
}

/// Lines up the comments at the ends of consecutive lines, one space after
/// the longest line's code:
///
/// ```text
/// print(add5(1));                       // expect: 6
/// print(compose(add5, |x| x + 100, 0)); // expect: 105
/// ```
///
/// Why line them up: in T's tests, and in examples, a run of statements with
/// `// expect:` after each reads as a table, code on the left and what it
/// prints on the right, and that only works if the right column is
/// straight. Why only consecutive lines: a line without a comment, or a
/// blank one, ends the run (as in gofmt), so one long line moves the
/// comments of its own run and no others, and an edit causes a small diff.
///
/// A line that would push a comment in its run past `WIDTH` starts a new
/// run instead (as in clang-format), so lining comments up never makes a
/// line too long that wasn't already. A comment alone on its line in a run,
/// continuing the one above, stays in that run.
fn align_comments(lines: &mut [OutLine]) {
    let code_width = |l: &OutLine| l.code.trim_end().chars().count();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].comment.is_none() {
            i += 1;
            continue;
        }
        // The run is i..end, its comments go at `column`.
        let mut column = code_width(&lines[i]) + 1;
        let mut longest = 0;
        let mut end = i;
        while let Some(line) = lines.get(end)
            && let Some(comment) = &line.comment
        {
            let continues = line.code.trim().is_empty();
            let col = if continues {
                column
            } else {
                column.max(code_width(line) + 1)
            };
            let len = longest.max(comment.chars().count());
            if end > i && !continues && col + len > WIDTH {
                break;
            }
            (column, longest, end) = (col, len, end + 1);
        }
        for line in &mut lines[i..end] {
            let comment = line.comment.take().unwrap();
            let code = line.code.trim_end();
            line.code = format!("{code:column$}{comment}");
        }
        i = end;
    }
}

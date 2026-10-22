//! What `t fmt` has to keep that isn't in the AST: comments, and the blank
//! lines between things.
//!
//! The printer walks the AST in source order and, at each place a comment
//! can go, asks `Trivia` for the comments up to there. Each comment is
//! handed out once, in order, so none is lost or duplicated; one that sits
//! somewhere the printer never asks about (inside `a + b`, say) comes out at
//! the next place it does ask, on a line of its own.
//!
//! A `///` that documents nothing (`Program::stray_docs`) is trivia too,
//! but moving one can make it document something: before a field, it
//! would document the field. So inside a struct or enum, they're held back
//! until its `}` (`hold_docs`).

use crate::error::Span;
use crate::lexer::{Comment, Tok, Token};

pub struct Trivia<'s> {
    src: &'s str,
    toks: Vec<Token>,
    /// The comments, in the order they're printed, each with the place in
    /// the source to print it at: where it is, unless it's held back.
    comments: Vec<(usize, Comment)>,
    /// The first comment not printed yet.
    next: usize,
    /// Where each comment is in the source, in order.
    spans: Vec<Span>,
}

/// A comment after code on the same line, with any comments lined up under
/// it on the lines that follow:
///
/// ```text
/// print(h(true));   // expect: 1
///                   // expect: 2
/// ```
pub struct Trailing {
    pub text: String,
    pub more: Vec<String>,
}

/// A comment on a line of its own.
pub struct Leading {
    pub text: String,
    /// Whether a blank line comes before it.
    pub blank: bool,
}

impl<'s> Trivia<'s> {
    pub fn new(src: &'s str, toks: Vec<Token>, comments: Vec<Comment>) -> Trivia<'s> {
        Trivia {
            src,
            toks,
            spans: comments.iter().map(|c| c.span).collect(),
            comments: comments.into_iter().map(|c| (c.span.start, c)).collect(),
            next: 0,
        }
    }

    /// Holds back the doc comments not printed yet that are between `from`
    /// and `to`, the `}` that ends a struct or enum, to print there, where
    /// they can't document a field or variant.
    pub fn hold_docs(&mut self, from: usize, to: usize) {
        let src = self.src;
        let is_doc = |c: &Comment| {
            let text = &src[c.span.start..c.span.end];
            text.starts_with("///") && !text.starts_with("////")
        };
        for (at, c) in &mut self.comments[self.next..] {
            if from <= c.span.start && c.span.start < to && is_doc(c) {
                *at = to - 1;
            }
        }
        // Stable, so comments printed at the same place keep their order.
        self.comments[self.next..].sort_by_key(|(at, _)| *at);
    }

    fn text(&self, c: &Comment) -> String {
        self.src[c.span.start..c.span.end].to_string()
    }

    /// Is nothing but whitespace before `c` on its line?
    fn own_line(&self, c: &Comment) -> bool {
        let line_start = self.src[..c.span.start].rfind('\n').map_or(0, |i| i + 1);
        self.src[line_start..c.span.start].trim().is_empty()
    }

    fn column(&self, offset: usize) -> usize {
        let line_start = self.src[..offset].rfind('\n').map_or(0, |i| i + 1);
        self.src[line_start..offset].chars().count()
    }

    /// The index of the first token starting at or after `pos`.
    fn tok_index(&self, pos: usize) -> usize {
        self.toks.partition_point(|t| t.span.start < pos)
    }

    /// Are there comments not printed yet that start before `pos`?
    pub fn any_before(&self, pos: usize) -> bool {
        self.comments
            .get(self.next)
            .is_some_and(|(at, _)| *at < pos)
    }

    /// Takes the comments not printed yet that start before `pos`.
    pub fn leading(&mut self, pos: usize) -> Vec<Leading> {
        let mut out = Vec::new();
        while let Some((at, c)) = self.comments.get(self.next)
            && *at < pos
        {
            out.push(Leading {
                text: self.text(c),
                blank: self.blank_before(c.span.start),
            });
            self.next += 1;
        }
        out
    }

    /// Takes the comment after the code that ends at `end`, if it's on the
    /// same line with nothing but `,` or `;` between.
    pub fn trailing(&mut self, end: usize) -> Option<Trailing> {
        let (at, c) = self.comments.get(self.next)?;
        if *at != c.span.start
            || c.span.start < end
            || self.own_line(c)
            || self.src[end..c.span.start].contains('\n')
        {
            return None;
        }
        let between = &self.toks[self.tok_index(end)..self.tok_index(c.span.start)];
        if !between
            .iter()
            .all(|t| matches!(t.tok, Tok::Comma | Tok::Semi))
        {
            return None;
        }
        let text = self.text(c);
        let column = self.column(c.span.start);
        let mut prev_end = c.span.end;
        self.next += 1;

        let mut more = Vec::new();
        while let Some((at, c)) = self.comments.get(self.next)
            && *at == c.span.start
            && self.src[prev_end..c.span.start].matches('\n').count() == 1
            && self.own_line(c)
            && self.column(c.span.start) == column
        {
            more.push(self.text(c));
            prev_end = c.span.end;
            self.next += 1;
        }
        Some(Trailing { text, more })
    }

    /// Is there a blank line between `pos` and the token or comment before
    /// it?
    pub fn blank_before(&self, pos: usize) -> bool {
        let i = self.tok_index(pos);
        let tok_end = if i == 0 { 0 } else { self.toks[i - 1].span.end };
        let c = self.spans.partition_point(|c| c.start < pos);
        let comment_end = if c == 0 { 0 } else { self.spans[c - 1].end };
        let prev = tok_end.max(comment_end).min(pos);
        // Only whitespace in between, so two newlines make a blank line.
        prev > 0 && self.src[prev..pos].matches('\n').count() >= 2
    }

    /// Where the first `tok` at or after `pos` starts.
    pub fn find(&self, pos: usize, tok: &Tok) -> Span {
        self.toks[self.tok_index(pos)..]
            .iter()
            .find(|t| t.tok == *tok)
            .map_or(Span::new(self.src.len(), self.src.len()), |t| t.span)
    }

    /// How the integer literal at `span` is written, `0xFF_u8`, or the
    /// literal pattern, `-0x80..=0x7F`: its tokens, with nothing between
    /// them. `None` if there's anything else there.
    pub fn spelled(&self, span: Span) -> Option<String> {
        let mut out = String::new();
        for t in &self.toks[self.tok_index(span.start)..] {
            if t.span.start >= span.end {
                break;
            }
            match t.tok {
                Tok::Int(..) | Tok::Minus | Tok::DotDot | Tok::DotDotEq => {
                    out.push_str(&self.src[t.span.start..t.span.end]);
                }
                _ => return None,
            }
        }
        (!out.is_empty()).then_some(out)
    }

    /// The token just before the one at `pos`: the `fn` before a function's
    /// name, or the `let` before a pattern.
    pub fn before(&self, pos: usize) -> Span {
        let i = self.tok_index(pos);
        self.toks[i.saturating_sub(1)].span
    }
}

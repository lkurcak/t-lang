//! What `t fmt` has to keep that isn't in the AST: comments, and the blank
//! lines between things.
//!
//! The printer walks the AST in source order and, at each place a comment
//! can go, asks `Trivia` for the comments up to there. Each comment is
//! handed out once, in order, so none is lost or duplicated; one that sits
//! somewhere the printer never asks about (inside `a + b`, say) comes out at
//! the next place it does ask, on a line of its own.

use crate::error::Span;
use crate::lexer::{Comment, Tok, Token};

pub struct Trivia<'s> {
    src: &'s str,
    toks: Vec<Token>,
    comments: Vec<Comment>,
    /// The first comment not printed yet.
    next: usize,
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
            comments,
            next: 0,
        }
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
            .is_some_and(|c| c.span.start < pos)
    }

    /// Takes the comments not printed yet that start before `pos`.
    pub fn leading(&mut self, pos: usize) -> Vec<Leading> {
        let mut out = Vec::new();
        while let Some(c) = self.comments.get(self.next)
            && c.span.start < pos
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
        let c = self.comments.get(self.next)?;
        if c.span.start < end || self.own_line(c) || self.src[end..c.span.start].contains('\n') {
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
        while let Some(c) = self.comments.get(self.next)
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
        let c = self.comments.partition_point(|c| c.span.start < pos);
        let comment_end = if c == 0 {
            0
        } else {
            self.comments[c - 1].span.end
        };
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

    /// The token just before the one at `pos`: the `fn` before a function's
    /// name, or the `let` before a pattern.
    pub fn before(&self, pos: usize) -> Span {
        let i = self.tok_index(pos);
        self.toks[i.saturating_sub(1)].span
    }
}

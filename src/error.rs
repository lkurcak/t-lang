//! Source spans and diagnostics shared by every compiler phase.

/// A byte range `start..end` in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

/// Which phase rejected the program. Keeping these apart matters: a type error
/// is the checker refusing a program *before* it runs, a runtime error is a
/// well-typed program hitting something types don't rule out (yet), like
/// division by zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Syntax,
    Type,
    Runtime,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub phase: Phase,
    pub span: Span,
    pub message: String,
}

impl Error {
    pub fn syntax(span: Span, message: impl Into<String>) -> Error {
        Error { phase: Phase::Syntax, span, message: message.into() }
    }

    pub fn ty(span: Span, message: impl Into<String>) -> Error {
        Error { phase: Phase::Type, span, message: message.into() }
    }

    pub fn runtime(span: Span, message: impl Into<String>) -> Error {
        Error { phase: Phase::Runtime, span, message: message.into() }
    }

    /// Renders the error rustc-style, with the offending line underlined.
    pub fn render(&self, path: &str, src: &str) -> String {
        let phase = match self.phase {
            Phase::Syntax => "syntax error",
            Phase::Type => "type error",
            Phase::Runtime => "runtime error",
        };
        let start = self.span.start.min(src.len());
        let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = src[start..].find('\n').map_or(src.len(), |i| start + i);
        let line_no = src[..start].matches('\n').count() + 1;
        let col = src[line_start..start].chars().count() + 1;
        let width = src[start..self.span.end.clamp(start, line_end)].chars().count().max(1);

        let gutter = " ".repeat(line_no.to_string().len());
        format!(
            "{phase}: {msg}\n{gutter}--> {path}:{line_no}:{col}\n{gutter} |\n{line_no} | {text}\n{gutter} | {pad}{carets}",
            msg = self.message,
            text = &src[line_start..line_end],
            pad = " ".repeat(col - 1),
            carets = "^".repeat(width),
        )
    }
}

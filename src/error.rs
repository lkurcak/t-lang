//! Source spans and diagnostics shared by every compiler phase.

/// A byte range `start..end` in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    /// Is `offset` inside, counting the end?
    pub fn contains(self, offset: usize) -> bool {
        self.start <= offset && offset <= self.end
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
    /// Changes that would fix it, if there are any obvious ones.
    pub fixes: Vec<Fix>,
}

impl Error {
    pub fn syntax(span: Span, message: impl Into<String>) -> Error {
        Error::new(Phase::Syntax, span, message)
    }

    pub fn ty(span: Span, message: impl Into<String>) -> Error {
        Error::new(Phase::Type, span, message)
    }

    pub fn runtime(span: Span, message: impl Into<String>) -> Error {
        Error::new(Phase::Runtime, span, message)
    }

    fn new(phase: Phase, span: Span, message: impl Into<String>) -> Error {
        Error {
            phase,
            span,
            message: message.into(),
            fixes: Vec::new(),
        }
    }

    pub fn with_fix(mut self, fix: Fix) -> Error {
        self.fixes.push(fix);
        self
    }

    /// Renders the error rustc-style, with the offending line underlined.
    pub fn render(&self, path: &str, src: &str) -> String {
        let label = match self.phase {
            Phase::Syntax => "syntax error",
            Phase::Type => "type error",
            Phase::Runtime => "runtime error",
        };
        render(label, self.span, &self.message, &self.fixes, path, src)
    }
}

/// Something legal but almost certainly not what was meant, like a `match`
/// arm that can never run. Unlike an error, it doesn't stop the program: its
/// meaning is clear, just pointless.
#[derive(Debug, Clone)]
pub struct Warning {
    pub lint: Lint,
    pub span: Span,
    pub message: String,
    pub fixes: Vec<Fix>,
}

impl Warning {
    pub fn render(&self, path: &str, src: &str) -> String {
        render("warning", self.span, &self.message, &self.fixes, path, src)
    }
}

/// The kinds of warning. Each has a name, so that one day a program can turn
/// them off or into errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lint {
    /// An arm or or-pattern alternative that earlier patterns already cover.
    UnreachablePatterns,
    /// A range that earlier patterns cover one or both ends of.
    OverlappingRanges,
    /// Or-pattern alternatives that touch end to end, like `0..10 | 10..20`.
    MergeableRanges,
    /// A variable that nothing reads.
    UnusedVariables,
    /// A function `main` never reaches, or a struct, enum or variant it
    /// never constructs.
    DeadCode,
}

impl Lint {
    pub fn name(self) -> &'static str {
        match self {
            Lint::UnreachablePatterns => "unreachable_patterns",
            Lint::OverlappingRanges => "overlapping_ranges",
            Lint::MergeableRanges => "mergeable_ranges",
            Lint::UnusedVariables => "unused_variables",
            Lint::DeadCode => "dead_code",
        }
    }
}

/// A change to the source that fixes a diagnostic, for editors to offer as a
/// quick fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    /// What it does, like "remove the unreachable arm".
    pub title: String,
    /// Non-overlapping edits, all made at once.
    pub edits: Vec<Edit>,
}

/// Replaces the text at `span` with `text`. An empty span inserts, and empty
/// text deletes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub text: String,
}

impl Fix {
    pub fn new(title: impl Into<String>, edits: Vec<Edit>) -> Fix {
        Fix {
            title: title.into(),
            edits,
        }
    }

    /// `src` with the edits made.
    pub fn apply(&self, src: &str) -> String {
        let mut edits: Vec<&Edit> = self.edits.iter().collect();
        edits.sort_by_key(|e| std::cmp::Reverse(e.span.start));
        let mut out = src.to_string();
        for e in edits {
            out.replace_range(e.span.start..e.span.end, &e.text);
        }
        out
    }
}

impl Edit {
    pub fn replace(span: Span, text: impl Into<String>) -> Edit {
        Edit {
            span,
            text: text.into(),
        }
    }

    pub fn insert(at: usize, text: impl Into<String>) -> Edit {
        Edit::replace(Span::new(at, at), text)
    }

    pub fn delete(span: Span) -> Edit {
        Edit::replace(span, "")
    }
}

/// A diagnostic rustc-style: the offending line underlined, then a `help`
/// line for each fix.
fn render(label: &str, span: Span, message: &str, fixes: &[Fix], path: &str, src: &str) -> String {
    let start = span.start.min(src.len());
    let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[start..].find('\n').map_or(src.len(), |i| start + i);
    let line_no = src[..start].matches('\n').count() + 1;
    let col = src[line_start..start].chars().count() + 1;
    let width = src[start..span.end.clamp(start, line_end)]
        .chars()
        .count()
        .max(1);

    let gutter = " ".repeat(line_no.to_string().len());
    let mut out = format!(
        "{label}: {message}\n{gutter}--> {path}:{line_no}:{col}\n{gutter} |\n{line_no} | {text}\n{gutter} | {pad}{carets}",
        text = &src[line_start..line_end],
        pad = " ".repeat(col - 1),
        carets = "^".repeat(width),
    );
    for fix in fixes {
        out.push_str(&format!("\n{gutter} = help: {}", fix.title));
    }
    out
}

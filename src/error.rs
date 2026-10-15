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
    /// Other places that explain it, each with what happens there: "`x`
    /// is borrowed here".
    pub notes: Vec<(Span, String)>,
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
            notes: Vec::new(),
        }
    }

    pub fn with_fix(mut self, fix: Fix) -> Error {
        self.fixes.push(fix);
        self
    }

    pub fn with_note(mut self, span: Span, note: impl Into<String>) -> Error {
        self.notes.push((span, note.into()));
        self
    }

    /// Renders the error rustc-style, with the offending line underlined.
    pub fn render(&self, path: &str, src: &str) -> String {
        let label = match self.phase {
            Phase::Syntax => "syntax error",
            Phase::Type => "type error",
            Phase::Runtime => "runtime error",
        };
        let mut out = render(
            label,
            self.span,
            &self.message,
            &self.fixes,
            None,
            path,
            src,
        );
        for (span, note) in &self.notes {
            out.push('\n');
            out.push_str(&render("note", *span, note, &[], None, path, src));
        }
        out
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
    /// Renders the warning rustc-style. With `note`, it ends by saying how
    /// to turn its lint off, as it should the first time the lint comes up.
    pub fn render(&self, path: &str, src: &str, note: bool) -> String {
        let note = note.then(|| format!("`#[allow({})]` turns this off", self.lint.name()));
        let (span, fixes) = (self.span, &self.fixes);
        render("warning", span, &self.message, fixes, note, path, src)
    }

    /// Renders each of `warnings`, noting how to turn a lint off the first
    /// time it comes up, as rustc does.
    pub fn render_all(warnings: &[Warning], path: &str, src: &str) -> Vec<String> {
        let mut seen = Vec::new();
        warnings
            .iter()
            .map(|w| {
                let first = !seen.contains(&w.lint);
                seen.push(w.lint);
                w.render(path, src, first)
            })
            .collect()
    }
}

/// The kinds of warning. Each has a name, for `#[allow(..)]` to turn it off
/// with (see `allow.rs`).
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
    /// A `///` doc comment that isn't before anything it can document.
    UnusedDocComments,
    /// A struct, enum, variant or type parameter whose name isn't in
    /// `UpperCamelCase`.
    NonCamelCaseTypes,
    /// A function, parameter, variable or field whose name isn't in
    /// `snake_case`.
    NonSnakeCase,
    /// A name in `#[allow(..)]` that isn't a lint's.
    UnknownLints,
    /// A variable declared `mut` that's never changed.
    UnusedMut,
}

impl Lint {
    pub fn name(self) -> &'static str {
        match self {
            Lint::UnreachablePatterns => "unreachable_patterns",
            Lint::OverlappingRanges => "overlapping_ranges",
            Lint::MergeableRanges => "mergeable_ranges",
            Lint::UnusedVariables => "unused_variables",
            Lint::DeadCode => "dead_code",
            Lint::UnusedDocComments => "unused_doc_comments",
            Lint::NonCamelCaseTypes => "non_camel_case_types",
            Lint::NonSnakeCase => "non_snake_case",
            Lint::UnknownLints => "unknown_lints",
            Lint::UnusedMut => "unused_mut",
        }
    }

    pub const ALL: [Lint; 10] = [
        Lint::UnreachablePatterns,
        Lint::OverlappingRanges,
        Lint::MergeableRanges,
        Lint::UnusedVariables,
        Lint::DeadCode,
        Lint::UnusedDocComments,
        Lint::NonCamelCaseTypes,
        Lint::NonSnakeCase,
        Lint::UnknownLints,
        Lint::UnusedMut,
    ];

    pub fn from_name(name: &str) -> Option<Lint> {
        Lint::ALL.into_iter().find(|l| l.name() == name)
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
/// line for each fix, and the `note`, if any.
fn render(
    label: &str,
    span: Span,
    message: &str,
    fixes: &[Fix],
    note: Option<String>,
    path: &str,
    src: &str,
) -> String {
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
    if let Some(note) = note {
        out.push_str(&format!("\n{gutter} = note: {note}"));
    }
    out
}

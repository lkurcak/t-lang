//! The files of a program: its first, and one for each `mod m;` (see
//! `load.rs`). They're laid end to end in one text, a newline between each
//! two, so a `Span` is still a range in one text: every phase works on it as
//! on a single file, and only messages and editors ask which file an offset
//! is in. No span crosses from one file into another, since no token does.

use crate::error::{Edit, Span};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Sources {
    text: String,
    files: Vec<File>,
}

#[derive(Debug, Clone)]
pub struct File {
    /// The first file's as given, and each other's from its directory:
    /// `dir/a/b.t` for module `a::b` of `dir/main.t`.
    pub path: PathBuf,
    /// Where it starts in the text.
    pub start: usize,
    /// Its length.
    pub len: usize,
    /// The module it holds, by its index in `Program::mods`.
    pub module: usize,
    /// The name in the `mod m;` it's the file of: none for the first.
    pub declared: Option<Span>,
}

impl File {
    pub fn span(&self) -> Span {
        Span::new(self.start, self.start + self.len)
    }
}

impl Sources {
    /// A program of one file.
    pub fn single(path: impl Into<PathBuf>, src: &str) -> Sources {
        let mut sources = Sources {
            text: String::new(),
            files: Vec::new(),
        };
        sources.add(path.into(), src, crate::ast::ROOT, None);
        sources
    }

    /// Adds a file, holding `module`, named at `declared`, and returns
    /// where it starts.
    pub(crate) fn add(
        &mut self,
        path: PathBuf,
        src: &str,
        module: usize,
        declared: Option<Span>,
    ) -> usize {
        if !self.files.is_empty() {
            self.text.push('\n');
        }
        let start = self.text.len();
        self.text.push_str(src);
        self.files.push(File {
            path,
            start,
            len: src.len(),
            module,
            declared,
        });
        start
    }

    /// Every file, laid end to end.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Every file, in the order they're in the text: the first one first.
    pub fn files(&self) -> &[File] {
        &self.files
    }

    /// The file `offset` is in, or at the end of.
    pub fn file_at(&self, offset: usize) -> &File {
        let i = self.files.partition_point(|f| f.start <= offset);
        &self.files[i.saturating_sub(1)]
    }

    /// The file at `path`, if it's one of these.
    pub fn file(&self, path: &Path) -> Option<&File> {
        self.files.iter().find(|f| f.path == path)
    }

    /// A file's text.
    pub fn src(&self, file: &File) -> &str {
        &self.text[file.start..file.start + file.len]
    }

    /// `span`'s file, and where in it `span` is.
    pub fn local(&self, span: Span) -> (&File, Span) {
        let file = self.file_at(span.start);
        let end = span.end.min(file.start + file.len).max(span.start);
        (file, Span::new(span.start - file.start, end - file.start))
    }

    /// The line `offset` is on, for messages: `3`, or `3 of a/b.t` if it
    /// isn't in the first file.
    pub fn line(&self, offset: usize) -> String {
        let (file, at) = self.local(Span::new(offset, offset));
        let line = self.src(file)[..at.start].matches('\n').count() + 1;
        match file.start {
            0 => line.to_string(),
            _ => format!("{line} of {}", file.path.display()),
        }
    }

    /// Each file's text with `edits` made, in the order of `files`. An edit
    /// is in the file its span starts in.
    pub fn edited(&self, edits: &[Edit]) -> Vec<String> {
        self.files
            .iter()
            .map(|f| crate::error::Fix::new("", self.edits_in(f, edits)).apply(self.src(f)))
            .collect()
    }

    /// The edits in `file`, with spans in it.
    pub fn edits_in(&self, file: &File, edits: &[Edit]) -> Vec<Edit> {
        edits
            .iter()
            .filter(|e| self.file_at(e.span.start).start == file.start)
            .map(|e| Edit::replace(self.local(e.span).1, e.text.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The end of a file is in it, not in the next: an edit inserting
    /// there, like a missing `}`, belongs to it.
    #[test]
    fn a_file_ends_before_the_next_starts() {
        let mut sources = Sources::single("main.t", "mod a;");
        let start = sources.add("a.t".into(), "fn f() {}", 1, None);
        let end = Span::new(6, 6);
        assert_eq!(sources.file_at(6).path, Path::new("main.t"));
        assert_eq!(sources.local(end).1, end);
        assert_eq!(sources.file_at(start).path, Path::new("a.t"));
        assert_eq!(
            sources.local(Span::new(start, start + 2)).1,
            Span::new(0, 2)
        );
        let edits = [Edit::insert(6, "x"), Edit::insert(start, "y")];
        assert_eq!(sources.edited(&edits), ["mod a;x", "yfn f() {}"]);
        assert_eq!(sources.line(start + 3), "1 of a.t");
    }
}

//! Reading a program's files: its first, then the file of each `mod m;`.
//!
//! The file of module `a::b` is `a/b.t`, from the first file's directory,
//! whether `b` is `mod b;` in `a.t` or in `mod a { .. }` (see
//! docs/06d.1-files.md). Only a program that starts in `main.t` has such
//! files, and no module in a file is called `main`, so every `main.t` is
//! where a program starts, and a file's program is the nearest `main.t`
//! above it that has it.
//!
//! `mod m;` means just what `mod m { .. }` would with the file's items in
//! the braces: modules are hermetic, so where a module's items are written
//! can't change what they mean. Loading ends, since each file is for a
//! module path of its own, and no file is read twice: not even through a
//! link, which could otherwise make `a/a/a/..` go on forever.

use crate::ast::Program;
use crate::error::{Error, Span};
use crate::lexer;
use crate::parser;
use crate::source::Sources;
use std::path::{Path, PathBuf};

/// Reads a file, for `load`. Tools can give the text of files open in an
/// editor rather than what's on disk.
pub type Read<'r> = &'r dyn Fn(&Path) -> std::io::Result<String>;

/// Reads the file system.
pub fn read_file(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

/// Parses the program that starts at `path`, whose text is `src`, and the
/// files of its modules, read with `read`. The sources are what was read,
/// as far as it got.
pub fn load(path: &Path, src: &str, read: Read) -> (Sources, Result<Program, Error>) {
    let mut sources = Sources::single(path, src);
    let result = files(path, &mut sources, read);
    (sources, result)
}

fn files(path: &Path, sources: &mut Sources, read: Read) -> Result<Program, Error> {
    let mut prog = lexer::lex(sources.text()).and_then(parser::parse)?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let starts_in_main = path.file_name().is_some_and(|n| n == "main.t");
    let mut seen = vec![canonical(path)];
    // Modules are added as files are parsed, so this sees those too.
    let mut m = 0;
    while m < prog.mods.len() {
        if !prog.mods[m].in_file {
            m += 1;
            continue;
        }
        let decl = &prog.mods[m];
        let name = &decl.name.name;
        if !starts_in_main {
            let first = path
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
            return Err(Error::syntax(
                decl.span,
                format!(
                    "`mod {name};` is a module in a file of its own, which only a program that \
                     starts in `main.t` can have, and this one starts in `{first}`; write its \
                     items here, in `mod {name} {{ .. }}`"
                ),
            ));
        }
        if name == "main" {
            return Err(Error::syntax(
                decl.name.span,
                "a module in a file of its own can't be called `main`: a file `main.t` is \
                 where a program starts",
            ));
        }
        let segments = module_path(&prog, m);
        let file = dir.join(segments.join("/") + ".t");
        let shown = segments.join("::");
        let text = read(&file).map_err(|e| {
            Error::syntax(
                decl.span,
                format!(
                    "can't read `{}`, the file of module `{shown}`: {e}",
                    file.display()
                ),
            )
        })?;
        let id = canonical(&file);
        if seen.contains(&id) {
            return Err(Error::syntax(
                decl.span,
                format!(
                    "`{}`, the file of module `{shown}`, is a file already read, through a link",
                    file.display()
                ),
            ));
        }
        seen.push(id);
        let start = sources.add(file, &text, m, Some(prog.mods[m].name.span));
        let toks = lexer::lex(&text).map_err(|e| shift(e, start))?;
        let toks = toks
            .into_iter()
            .map(|mut t| {
                t.span = moved(t.span, start);
                t
            })
            .collect();
        parser::parse_into(toks, &mut prog, m)?;
        m += 1;
    }
    Ok(prog)
}

/// The names of the modules from the crate root down to `m`.
fn module_path(prog: &Program, mut m: usize) -> Vec<&str> {
    let mut segments = Vec::new();
    while let Some(parent) = prog.mods[m].parent {
        segments.push(prog.mods[m].name.name.as_str());
        m = parent;
    }
    segments.reverse();
    segments
}

/// The file `path` really is, as far as can be told: a file not on disk,
/// open in an editor, is only what it's called.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn moved(span: Span, by: usize) -> Span {
    Span::new(span.start + by, span.end + by)
}

/// An error of lexing a file on its own, with its spans where the file is.
fn shift(mut e: Error, by: usize) -> Error {
    e.span = moved(e.span, by);
    for (span, _) in &mut e.notes {
        *span = moved(*span, by);
    }
    for fix in &mut e.fixes {
        for edit in &mut fix.edits {
            edit.span = moved(edit.span, by);
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program of files means what it would with each file's items in
    /// the braces of `mod m { .. }`: it monomorphises to the same items,
    /// though in another order, as files are parsed after the first.
    #[test]
    fn files_mean_what_inline_modules_do() {
        let files = [
            ("p/a.t", "pub mod b;\npub fn f() -> int { b::g() + 1 }\n"),
            ("p/a/b.t", "pub fn g() -> int { crate::c::d::h() }\n"),
            ("p/c/d.t", "pub fn h() -> int { 41 }\n"),
        ];
        let read = |p: &Path| match files.iter().find(|(f, _)| Path::new(f) == p) {
            Some((_, text)) => Ok(text.to_string()),
            None => crate::no_files(p),
        };
        let main = "mod a;\nmod c { pub mod d; }\nfn main() -> int { a::f() }\n";
        let split = crate::check_files(Path::new("p/main.t"), main, &read);
        let inline = "mod a {\n    pub mod b { pub fn g() -> int { crate::c::d::h() } }\n    \
                      pub fn f() -> int { b::g() + 1 }\n}\n\
                      mod c { pub mod d { pub fn h() -> int { 41 } } }\n\
                      fn main() -> int { a::f() }\n";
        let inline = crate::check_source(inline);
        let shown = |c: crate::Checked| {
            let mut items: Vec<String> = c
                .result
                .unwrap()
                .to_string()
                .split_terminator("\n\n")
                .map(str::trim)
                .map(String::from)
                .collect();
            items.sort();
            items
        };
        assert_eq!(shown(split), shown(inline));
    }

    /// A link can't make a module's file one already read, which would
    /// otherwise go on forever: `a/a/a/..`.
    #[cfg(unix)]
    #[test]
    fn a_link_cant_read_a_file_twice() {
        let dir = std::env::temp_dir().join(format!("t-load-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.t"), "mod a;\n").unwrap();
        std::os::unix::fs::symlink(&dir, dir.join("a")).unwrap();
        let main = dir.join("main.t");
        let (_, result) = load(&main, "mod a;\nfn main() {}\n", &read_file);
        std::fs::remove_dir_all(&dir).unwrap();
        let message = result.err().map(|e| e.message).unwrap_or_default();
        assert!(message.contains("a file already read"), "{message}");
    }
}

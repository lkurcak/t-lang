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
//!
//! Every program has the core library too (see docs/06e-core.md): a module
//! tree of its own beside the crate root's, read before it, from the files
//! of `core/` that are built into the compiler. Its root is `lib.t`, and
//! the file of its module `a::b` is `a/b.t` there. Its paths are shown as
//! `<core>/a/b.t`. The alloc library is the same, read next, from `alloc/`
//! (see docs/09-unsafe.md).

use crate::ast::{Ident, ModDecl, Program, ROOT};
use crate::error::{Error, Span};
use crate::lexer;
use crate::parser;
use crate::source::Sources;
use std::path::{Path, PathBuf};

/// Reads a file, for `load`. Tools can give the text of files open in an
/// editor rather than what's on disk.
pub type Read<'r> = &'r dyn Fn(&Path) -> std::io::Result<String>;

/// The files of the core library, by their paths in `core/`.
pub const CORE_FILES: &[(&str, &str)] = &[
    ("lib.t", include_str!("../core/lib.t")),
    ("char.t", include_str!("../core/char.t")),
    ("cmp.t", include_str!("../core/cmp.t")),
    ("iter.t", include_str!("../core/iter.t")),
    ("mem.t", include_str!("../core/mem.t")),
    ("num.t", include_str!("../core/num.t")),
    ("ops.t", include_str!("../core/ops.t")),
    ("option.t", include_str!("../core/option.t")),
    ("panic.t", include_str!("../core/panic.t")),
    ("prelude.t", include_str!("../core/prelude.t")),
    ("result.t", include_str!("../core/result.t")),
    ("slice.t", include_str!("../core/slice.t")),
    ("str.t", include_str!("../core/str.t")),
];

/// Where the core library's files seem to be, in messages and to editors.
pub const CORE_DIR: &str = "<core>";

/// The files of the alloc library, by their paths in `alloc/`.
pub const ALLOC_FILES: &[(&str, &str)] = &[
    ("lib.t", include_str!("../alloc/lib.t")),
    ("list.t", include_str!("../alloc/list.t")),
    ("raw.t", include_str!("../alloc/raw.t")),
    ("string.t", include_str!("../alloc/string.t")),
];

/// Where the alloc library's files seem to be.
pub const ALLOC_DIR: &str = "<alloc>";

/// A library built into the compiler: its name, its files and where they
/// seem to be.
pub(crate) type Library = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static str,
);

/// The libraries built into the compiler, in the order they're read.
pub(crate) const LIBRARIES: [Library; 2] = [
    ("core", CORE_FILES, CORE_DIR),
    ("alloc", ALLOC_FILES, ALLOC_DIR),
];

/// Reads the file system.
pub fn read_file(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

/// Parses the program that starts at `path`, whose text is `src`, and the
/// files of its modules, read with `read`. The sources are what was read,
/// as far as it got: the libraries' first, then the program's.
///
/// The libraries are as checking them left them (see `check::libraries`),
/// with their paths resolved: they're read and checked only once.
pub fn load(path: &Path, src: &str, read: Read) -> (Sources, Result<Program, Error>) {
    let libraries = crate::check::libraries();
    let (mut sources, mut prog) = (libraries.sources.clone(), libraries.prog.clone());
    let result = files(path, src, &mut prog, &mut sources, read).map(|()| prog);
    (sources, result)
}

/// No files, and a program of nothing but an empty crate root, which a
/// program's first file fills: what reading the libraries starts with.
pub(crate) fn empty() -> (Sources, Program) {
    let prog = lexer::lex("")
        .and_then(parser::parse)
        .expect("an empty program parses");
    (Sources::default(), prog)
}

/// Reads `library` into `prog`, after what's there, and returns its root
/// module.
pub(crate) fn library(
    prog: &mut Program,
    sources: &mut Sources,
    library: Library,
) -> Result<usize, Error> {
    let m = prog.mods.len();
    add_library(prog, sources, library)?;
    library_files(prog, sources, m)?;
    Ok(m)
}

/// Reads the files of the modules of the library whose root is module
/// `m`, and of those after it, which are its.
fn library_files(prog: &mut Program, sources: &mut Sources, mut m: usize) -> Result<(), Error> {
    while m < prog.mods.len() {
        if prog.mods[m].in_file {
            let segments = module_path(prog, m);
            let (name, files, dir) = LIBRARIES
                .into_iter()
                .find(|(name, ..)| *name == segments[0])
                .expect("a library's module is in its tree");
            let file = segments[1..].join("/") + ".t";
            let Some(&(_, text)) = files.iter().find(|(f, _)| *f == file) else {
                unreachable!("the {name} library has no file `{file}`");
            };
            let path = Path::new(dir).join(&file);
            let start = sources.add(path, text, m, Some(prog.mods[m].name.span));
            parse_file(prog, m, text, start)?;
        }
        m += 1;
    }
    Ok(())
}

/// Parses the program's first file, at `path`, whose text is `src`, into
/// the crate root of `prog`, and then the files of its modules.
fn files(
    path: &Path,
    src: &str,
    prog: &mut Program,
    sources: &mut Sources,
    read: Read,
) -> Result<(), Error> {
    let start = sources.add(path.to_path_buf(), src, ROOT, None);
    prog.mods[ROOT].span = Span::new(start, start + src.len());
    prog.mods[ROOT].name.span = Span::new(start, start);
    let first = prog.mods.len();
    parse_file(prog, ROOT, src, start)?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let starts_in_main = path.file_name().is_some_and(|n| n == "main.t");
    let mut seen = vec![canonical(path)];
    // Modules are added as files are parsed, so this sees those too.
    let mut m = first;
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
        let segments = module_path(prog, m);
        // A directory for each module it's in, joined as paths are where
        // it runs: `a\b.t` on Windows, not `a/b.t`.
        let (last, outer) = segments.split_last().expect("a module has a name");
        let mut file = dir.to_path_buf();
        file.extend(outer);
        file.push(format!("{last}.t"));
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
        parse_file(prog, m, &text, start)?;
        m += 1;
    }
    Ok(())
}

/// Parses `text`, which starts at `start` in the sources, into module `m`.
fn parse_file(prog: &mut Program, m: usize, text: &str, start: usize) -> Result<(), Error> {
    let toks = lexer::lex(text).map_err(|e| shift(e, start))?;
    let toks = toks
        .into_iter()
        .map(|mut t| {
            t.span = moved(t.span, start);
            t
        })
        .collect();
    parser::parse_into(toks, prog, m)
}

/// Adds a library's root module, from its `lib.t`, which is parsed here;
/// its modules' files are read after.
fn add_library(
    prog: &mut Program,
    sources: &mut Sources,
    (name, files, dir): Library,
) -> Result<(), Error> {
    let root = prog.mods.len();
    let text = files[0].1;
    let start = sources.add(Path::new(dir).join("lib.t"), text, root, None);
    let span = Span::new(start, start + text.len());
    let id = prog.fresh_id();
    prog.mods.push(ModDecl {
        doc: None,
        allows: Vec::new(),
        span,
        name: Ident {
            name: name.into(),
            span: Span::new(start, start),
            id,
        },
        parent: Some(ROOT),
        public: Some(Span::new(start, start)),
        in_file: false,
    });
    match name {
        "core" => prog.core = Some(root),
        _ => prog.alloc = Some(root),
    }
    parse_file(prog, root, text, start)
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
            ("p/a.t", "pub mod b;\npub fn f() -> i64 { b::g() }\n"),
            ("p/a/b.t", "pub fn g() -> i64 { crate::c::d::h() }\n"),
            ("p/c/d.t", "pub fn h() -> i64 { 42 }\n"),
        ];
        let read = |p: &Path| match files.iter().find(|(f, _)| Path::new(f) == p) {
            Some((_, text)) => Ok(text.to_string()),
            None => crate::no_files(p),
        };
        let main = "mod a;\nmod c { pub mod d; }\nfn main() -> i64 { a::f() }\n";
        let split = crate::check_files(Path::new("p/main.t"), main, &read);
        let inline = "mod a {\n    pub mod b { pub fn g() -> i64 { crate::c::d::h() } }\n    \
                      pub fn f() -> i64 { b::g() }\n}\n\
                      mod c { pub mod d { pub fn h() -> i64 { 42 } } }\n\
                      fn main() -> i64 { a::f() }\n";
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

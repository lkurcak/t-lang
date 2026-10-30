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

use crate::ast::{Ident, ModDecl, Package, Program, ROOT, Vis};
use crate::error::{Error, Span};
use crate::lexer;
use crate::parser;
use crate::source::{Kind, Sources};
use std::path::{Path, PathBuf};

/// Reads a file, for `load`. Tools can give the text of files open in an
/// editor rather than what's on disk.
pub type Read<'r> = &'r dyn Fn(&Path) -> std::io::Result<String>;

/// The files of the core library, by their paths in `core/`.
pub const CORE_FILES: &[(&str, &str)] = &[
    ("lib.t", include_str!("../core/lib.t")),
    ("array.t", include_str!("../core/array.t")),
    ("char.t", include_str!("../core/char.t")),
    ("cmp.t", include_str!("../core/cmp.t")),
    ("fmt.t", include_str!("../core/fmt.t")),
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
    ("arena.t", include_str!("../alloc/arena.t")),
    ("boxed.t", include_str!("../alloc/boxed.t")),
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

/// The files of the std library, by their paths in `std/`.
pub const STD_FILES: &[(&str, &str)] = &[
    ("lib.t", include_str!("../std/lib.t")),
    ("io.t", include_str!("../std/io.t")),
    ("rt.t", include_str!("../std/rt.t")),
];

/// Where the std library's files seem to be.
pub const STD_DIR: &str = "<std>";

/// The libraries built into the compiler, in the order they're read.
pub(crate) const LIBRARIES: [Library; 3] = [
    ("core", CORE_FILES, CORE_DIR),
    ("alloc", ALLOC_FILES, ALLOC_DIR),
    ("std", STD_FILES, STD_DIR),
];

/// Reads the file system.
pub fn read_file(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

/// Parses the program that starts at `path`, whose text is `src`, the
/// files of its modules, and the packages it depends on, read with `read`.
/// The sources are what was read, as far as it got: the libraries' first,
/// then the program's `t.toml`, its dependencies', and the program's own.
///
/// The libraries are as checking them left them (see `check::libraries`),
/// with their paths resolved: they're read and checked only once.
pub fn load(path: &Path, src: &str, read: Read) -> (Sources, Result<Program, Error>) {
    let libraries = crate::check::libraries();
    let (mut sources, mut prog) = (libraries.sources.clone(), libraries.prog.clone());
    let mut loader = Loader {
        prog: &mut prog,
        sources: &mut sources,
        read,
        top: path.parent().unwrap_or(Path::new("")).to_path_buf(),
        loaded: Vec::new(),
        loading: Vec::new(),
        entries: Vec::new(),
    };
    let result = loader.program(path, src);
    (sources, result.map(|()| prog))
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
            let start = sources.add(path, text, m, Some(prog.mods[m].name.span), Kind::Library);
            parse_file(prog, m, text, start)?;
        }
        m += 1;
    }
    Ok(())
}

/// Parses a package's first file, at `path`, whose text is `src`, which
/// starts at `start` in the sources, into its root module `root`, and then
/// the files of its modules, whose are `kind`.
#[allow(clippy::too_many_arguments)]
fn files(
    path: &Path,
    src: &str,
    start: usize,
    root: usize,
    kind: Kind,
    prog: &mut Program,
    sources: &mut Sources,
    read: Read,
) -> Result<(), Error> {
    let first = prog.mods.len();
    parse_file(prog, root, src, start)?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let starts_in_root = is_root_file(path);
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
        if !starts_in_root {
            let first = path
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
            return Err(Error::syntax(
                decl.span,
                format!(
                    "`mod {name};` is a module in a file of its own, which only a program that \
                     starts in `main.t`, or a library in `lib.t`, can have, and this one starts \
                     in `{first}`; write its items here, in `mod {name} {{ .. }}`"
                ),
            ));
        }
        if name == "main" || name == "lib" {
            let what = match name.as_str() {
                "main" => "program",
                _ => "library",
            };
            return Err(Error::syntax(
                decl.name.span,
                format!(
                    "a module in a file of its own can't be called `{name}`: a file `{name}.t` \
                     is where a {what} starts"
                ),
            ));
        }
        let mut segments = module_path(prog, m);
        // A dependency's modules' paths start with its name, which isn't a
        // directory of its.
        if root != ROOT {
            segments.remove(0);
        }
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
        let start = sources.add(file, &text, m, Some(prog.mods[m].name.span), kind);
        parse_file(prog, m, &text, start)?;
        m += 1;
    }
    Ok(())
}

/// Is `path` a file a package can start in, `main.t` or `lib.t`, which
/// can have modules in files and a `t.toml`?
fn is_root_file(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|n| n == "main.t" || n == "lib.t")
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
    let start = sources.add(
        Path::new(dir).join("lib.t"),
        text,
        root,
        None,
        Kind::Library,
    );
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
        vis: Vis::Export(Span::new(start, start)),
        in_file: false,
    });
    if name == "core" {
        prog.core = Some(root);
    }
    prog.libraries.push(root);
    parse_file(prog, root, text, start)
}

/// Reads a program's packages: the one it starts in, and those its
/// `t.toml` says it depends on, and theirs, each once (see
/// docs/26-packages.md).
///
/// A package is a directory, and which one is what makes two the same: it
/// is read once, whoever depends on it, so a type of its is one type
/// wherever it's named. Each is read after those it depends on, as each is
/// checked after them; one that would need itself first is an error, so
/// reading ends. They're named by their dependents, each package with one
/// name, which its items' symbols start with, so no two symbols are the
/// same.
struct Loader<'l> {
    prog: &'l mut Program,
    sources: &'l mut Sources,
    read: Read<'l>,
    /// The directory of the program's first file, which paths in messages
    /// are shown from.
    top: PathBuf,
    loaded: Vec<Loaded>,
    /// The directories of the packages being read, each depending on the
    /// next, and their names: none for the program's own.
    loading: Vec<(PathBuf, Option<String>)>,
    /// Where each package's `t.toml` names each package it depends on,
    /// in the order of `Program::packages`.
    entries: Vec<Vec<Span>>,
}

/// A package read, a dependency.
struct Loaded {
    dir: PathBuf,
    name: String,
    root: usize,
    /// The `t.toml` that first named it.
    by: PathBuf,
}

/// A dependency, as a `t.toml` says it.
struct Entry {
    name: String,
    at: Span,
    path: String,
}

impl Loader<'_> {
    /// Reads the program's packages, its own last.
    fn program(&mut self, path: &Path, src: &str) -> Result<(), Error> {
        let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
        let library = path.file_name().is_some_and(|n| n == "lib.t");
        let mut deps = Vec::new();
        let mut at = Vec::new();
        if is_root_file(path) {
            let other = dir.join(if library { "main.t" } else { "lib.t" });
            if (self.read)(&other).is_ok() {
                let start = self.add_root(path, src);
                let (this, that) = match library {
                    true => ("lib.t", "main.t"),
                    false => ("main.t", "lib.t"),
                };
                return Err(Error::syntax(
                    Span::new(start, start),
                    format!(
                        "this directory has a `{that}` as well as `{this}`; a package starts in \
                         one of them"
                    ),
                ));
            }
            self.loading.push((normal(&dir), None));
            for entry in self.manifest(&dir)? {
                deps.push((entry.name.clone(), self.dependency(&dir, &entry)?));
                at.push(entry.at);
            }
            self.loading.pop();
        }
        let from = self.prog.counts();
        let start = self.add_root(path, src);
        files(
            path,
            src,
            start,
            ROOT,
            Kind::Program,
            self.prog,
            self.sources,
            self.read,
        )?;
        self.prog.packages.push(Package {
            root: ROOT,
            from,
            deps,
            library,
        });
        self.entries.push(at);
        self.chain()
    }

    /// Adds the program's first file to the sources, as the crate root's.
    fn add_root(&mut self, path: &Path, src: &str) -> usize {
        let start = (self.sources).add(path.to_path_buf(), src, ROOT, None, Kind::Program);
        self.prog.mods[ROOT].span = Span::new(start, start + src.len());
        self.prog.mods[ROOT].name.span = Span::new(start, start);
        start
    }

    /// The dependencies the `t.toml` in `dir` lists, if there's one, in
    /// the order of their names.
    fn manifest(&mut self, dir: &Path) -> Result<Vec<Entry>, Error> {
        let path = dir.join("t.toml");
        let text = match (self.read)(&path) {
            Ok(text) => text,
            Err(_) => return Ok(Vec::new()),
        };
        let start = (self.sources).add(path.clone(), &text, ROOT, None, Kind::Manifest);
        let at = |r: std::ops::Range<usize>| Span::new(start + r.start, start + r.end);
        let shown = self.shown(&path);
        let table = toml::de::DeTable::parse(&text).map_err(|e| {
            let span = e.span().map_or(Span::new(start, start), at);
            let why = e.message().trim_end_matches('\n').to_string();
            Error::syntax(span, format!("`{shown}` isn't TOML: {why}"))
        })?;
        let mut entries = Vec::new();
        for (key, value) in table.get_ref() {
            if key.get_ref() != "dependencies" {
                return Err(Error::syntax(
                    at(key.span()),
                    format!(
                        "a `t.toml` has only `[dependencies]`, not `{}`",
                        key.get_ref()
                    ),
                ));
            }
            let toml::de::DeValue::Table(deps) = value.get_ref() else {
                return Err(Error::syntax(
                    at(value.span()),
                    "`dependencies` is a table, of a name for each package",
                ));
            };
            for (name, dep) in deps {
                let shape = || {
                    Error::syntax(
                        at(dep.span()),
                        format!(
                            "a dependency is a table with a `path`, as in `{} = {{ path = \
                             \"../{0}\" }}`",
                            name.get_ref()
                        ),
                    )
                };
                let toml::de::DeValue::Table(fields) = dep.get_ref() else {
                    return Err(shape());
                };
                let mut path = None;
                for (field, value) in fields {
                    match (field.get_ref().as_ref(), value.get_ref()) {
                        ("path", toml::de::DeValue::String(p)) => path = Some(p.to_string()),
                        ("path", _) => return Err(shape()),
                        (other, _) => {
                            return Err(Error::syntax(
                                at(field.span()),
                                format!("a dependency has only a `path`, not a `{other}`"),
                            ));
                        }
                    }
                }
                let Some(path) = path else {
                    return Err(shape());
                };
                entries.push(Entry {
                    name: name.get_ref().to_string(),
                    at: at(name.span()),
                    path,
                });
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Reads the package `entry` names, from the `t.toml` in `dir`, unless
    /// it's been read, and those it depends on first; and returns its root
    /// module.
    fn dependency(&mut self, dir: &Path, entry: &Entry) -> Result<usize, Error> {
        let name = &entry.name;
        let is_name = matches!(
            &lexer::lex(name).as_deref(),
            Ok([t, end]) if matches!(&t.tok, lexer::Tok::Ident(n) if n == name)
                && end.tok == lexer::Tok::Eof
        );
        if !is_name {
            return Err(Error::syntax(
                entry.at,
                format!("a package's name must be a name in T, as a module's is, not `{name}`"),
            ));
        }
        if LIBRARIES.iter().any(|(l, ..)| l == name) {
            return Err(Error::syntax(
                entry.at,
                format!(
                    "a package can't be called `{name}`: that's the name of the {name} library"
                ),
            ));
        }
        let target = normal(&dir.join(&entry.path));
        let id = canonical(&target);
        if let Some(i) = self.loading.iter().position(|(d, _)| canonical(d) == id) {
            let names: Vec<String> = (self.loading[i..].iter())
                .map(|(d, n)| match n {
                    Some(n) => format!("`{n}`"),
                    None => format!("the package in `{}`", self.shown(d)),
                })
                .collect();
            let mut how = names[0].clone();
            for n in names[1..].iter().chain(std::iter::once(&names[0])) {
                how.push_str(&format!(" depends on {n}, which"));
            }
            let how = how.trim_end_matches(", which");
            return Err(Error::syntax(
                entry.at,
                format!("packages can't depend on each other: {how}"),
            ));
        }
        let manifest = dir.join("t.toml");
        if let Some(l) = self.loaded.iter().find(|l| canonical(&l.dir) == id) {
            if l.name != *name {
                return Err(Error::syntax(
                    entry.at,
                    format!(
                        "the package in `{}` is called `{name}` here but `{}` in `{}`; call it \
                         the same everywhere",
                        self.shown(&l.dir),
                        l.name,
                        self.shown(&l.by)
                    ),
                ));
            }
            return Ok(l.root);
        }
        let lib = target.join("lib.t");
        let text = match (self.read)(&lib) {
            Ok(text) => text,
            Err(_) if (self.read)(&target.join("main.t")).is_ok() => {
                return Err(Error::syntax(
                    entry.at,
                    format!(
                        "`{}` is a program, with a `main.t`, not a library; a package to depend \
                         on starts in `lib.t`",
                        self.shown(&target)
                    ),
                ));
            }
            Err(e) => {
                return Err(Error::syntax(
                    entry.at,
                    format!(
                        "can't read `{}`, the first file of package `{name}`: {e}",
                        self.shown(&lib)
                    ),
                ));
            }
        };
        self.loading.push((target.clone(), Some(name.clone())));
        let mut deps = Vec::new();
        let mut at = Vec::new();
        for inner in self.manifest(&target)? {
            deps.push((inner.name.clone(), self.dependency(&target, &inner)?));
            at.push(inner.at);
        }
        self.loading.pop();
        if let Some(l) = self.loaded.iter().find(|l| l.name == *name) {
            return Err(Error::syntax(
                entry.at,
                format!(
                    "two packages are called `{name}`: the one in `{}` and the one in `{}`",
                    self.shown(&l.dir),
                    self.shown(&target)
                ),
            ));
        }
        let from = self.prog.counts();
        let root = self.prog.mods.len();
        let start = (self.sources).add(lib.clone(), &text, root, None, Kind::Dependency);
        let id = self.prog.fresh_id();
        self.prog.mods.push(ModDecl {
            doc: None,
            allows: Vec::new(),
            span: Span::new(start, start + text.len()),
            name: Ident {
                name: name.clone(),
                span: Span::new(start, start),
                id,
            },
            parent: Some(ROOT),
            vis: Vis::Export(Span::new(start, start)),
            in_file: false,
        });
        files(
            &lib,
            &text,
            start,
            root,
            Kind::Dependency,
            self.prog,
            self.sources,
            self.read,
        )?;
        self.prog.packages.push(Package {
            root,
            from,
            deps,
            library: true,
        });
        self.entries.push(at);
        self.loaded.push(Loaded {
            dir: target,
            name: name.clone(),
            root,
            by: manifest,
        });
        Ok(root)
    }

    /// Checks that the packages make a chain, each depending on the one
    /// before, as `Program::packages` has them in an order in which each
    /// is after those it depends on: then each package is checked against
    /// exactly those it depends on, as the order is the only one. Two that
    /// don't know each other could each keep a rule that spans packages,
    /// such as that proving what implements what ends, and break it
    /// together; for now that's ruled out (see docs/26-packages.md).
    fn chain(&self) -> Result<(), Error> {
        let packages = &self.prog.packages;
        let index = |root: usize| packages.iter().position(|p| p.root == root);
        // What each depends on, directly or not, by index.
        let mut below: Vec<Vec<bool>> = Vec::new();
        for p in packages {
            let mut row = vec![false; packages.len()];
            for &(_, d) in &p.deps {
                let d = index(d).expect("a dependency is read before its dependents");
                row[d] = true;
                for (k, &b) in below[d].iter().enumerate() {
                    row[k] |= b;
                }
            }
            below.push(row);
        }
        for i in 1..packages.len() {
            if below[i][i - 1] {
                continue;
            }
            // Where the two meet: the first package that depends on both,
            // which the program's own does.
            let (j, p) = (packages.iter().enumerate())
                .find(|&(j, _)| below[j][i - 1] && below[j][i])
                .expect("the program's own package depends on every other");
            let k = (p.deps.iter())
                .position(|&(_, d)| {
                    let d = index(d).expect("read");
                    d == i || below[d][i]
                })
                .expect("one of its dependencies brings it in");
            let name = |m: usize| self.prog.mods[packages[m].root].name.name.clone();
            return Err(Error::syntax(
                self.entries[j][k],
                format!(
                    "`{}` and `{}` don't depend on one another, which packages of one program \
                     must, for now: two that don't know each other could each keep the rules \
                     that span packages and break them together (see docs/26-packages.md)",
                    name(i - 1),
                    name(i)
                ),
            ));
        }
        Ok(())
    }

    /// `path` for messages: from the program's directory, if it's in it.
    fn shown(&self, path: &Path) -> String {
        let path = normal(path);
        let top = normal(&self.top);
        let shown = path.strip_prefix(&top).unwrap_or(&path);
        match shown.as_os_str().is_empty() {
            true => ".".to_string(),
            false => shown.to_string_lossy().replace('\\', "/"),
        }
    }
}

/// `path` with its `.` and `..` taken out, as far as it can be without
/// looking at the files: `a/b/../c` is `a/c`.
fn normal(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir
                if out
                    .components()
                    .next_back()
                    .is_some_and(|l| matches!(l, Component::Normal(_))) =>
            {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
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
        let main = "mod a;\nmod c { pub mod d; }\nfn main() { let _n = a::f(); }\n";
        let split = crate::check_files(Path::new("p/main.t"), main, &read);
        let inline = "mod a {\n    pub mod b { pub fn g() -> i64 { crate::c::d::h() } }\n    \
                      pub fn f() -> i64 { b::g() }\n}\n\
                      mod c { pub mod d { pub fn h() -> i64 { 42 } } }\n\
                      fn main() { let _n = a::f(); }\n";
        let inline = crate::check_source(inline);
        let shown = |c: crate::Checked| {
            let mut items: Vec<String> = c
                .result
                .unwrap()
                .lower()
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

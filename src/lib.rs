//! The T compiler. Pipeline: source → `lexer` → `parser` (for each file of
//! the program, as `load` reads them) → `check` (and `borrowck`, once each
//! function's types are known) → `lift` → `mono` → `interp`.
//!
//! The libraries built into the compiler, core, alloc and std, are read and
//! checked once, before any program (`check::libraries`), and each
//! program's reading and checking goes on from there (see
//! docs/14-packages.md).

mod allow;
pub mod ast;
mod borrowck;
pub mod cgen;
pub mod check;
mod consts;
mod doc;
pub mod error;
pub mod exhaust;
pub mod fmt;
pub mod interp;
pub mod intrinsic;
pub mod lexer;
pub mod lift;
pub mod load;
pub mod lsp;
pub(crate) mod mir;
pub mod mono;
mod naming;
pub mod parser;
mod pretty;
pub mod rename;
mod safety;
pub mod source;
mod target;
pub use target::Target;
pub mod traits;
mod trivia;
pub mod types;

use error::{Error, Warning};
use interp::Value;
use source::Sources;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Semantic analysis, including type, effect, refinement and borrow checking.
/// A successful analysis has not been lifted or monomorphised.
pub struct Checked {
    /// The program's files, as far as they were read.
    pub sources: Sources,
    /// Warnings found, including those before an error.
    pub warnings: Vec<Warning>,
    /// What the checker learned about the program, as far as it got.
    pub info: Arc<check::Info>,
    /// The checked program before executable lowering, or the first error.
    pub result: Result<CheckedProgram, Error>,
}

/// A program that passed semantic checking, paired with its semantic
/// information. Only analysis can construct one; later passes borrow it.
#[derive(Debug)]
pub struct CheckedProgram {
    program: ast::Program,
    info: Arc<check::Info>,
}

impl CheckedProgram {
    /// The checked AST, with nested functions and generics still present.
    pub fn program(&self) -> &ast::Program {
        &self.program
    }

    pub fn info(&self) -> &check::Info {
        &self.info
    }

    /// Lifts and monomorphises this program for execution or `t mono`.
    pub fn lower(&self) -> mono::Executable {
        mono::monomorphise(lift::lift(self))
    }
}

/// Analyses a program of one file, `main.t`, which has no `mod m;` files
/// to read. Lower a successful result explicitly to execute it.
pub fn check_source(src: &str) -> Checked {
    check_files(Path::new("main.t"), src, &no_files)
}

/// Reads no files, for a program of one.
pub fn no_files(path: &Path) -> std::io::Result<String> {
    let message = format!("only `main.t` is here, not `{}`", path.display());
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, message))
}

/// Analyses the program that starts at `path`, whose text is `src`, reading
/// the files of its modules with `read` (see `load.rs`).
pub fn check_files(path: &Path, src: &str, read: load::Read) -> Checked {
    check_files_for(path, src, read, Target::Hosted)
}

/// `check_files`, for `target`, which says which libraries the program has
/// (see docs/39-freestanding.md).
pub fn check_files_for(path: &Path, src: &str, read: load::Read, target: Target) -> Checked {
    let mut warnings = Vec::new();
    let (sources, program) = load::load_for(path, src, read, target);
    let mut program = match program {
        Ok(program) => program,
        Err(e) => {
            return Checked {
                sources,
                warnings,
                info: Arc::default(),
                result: Err(e),
            };
        }
    };
    let (info, result) = check::check(&mut program, &mut warnings);
    // Here rather than in `check`, since the fixes check renamed programs.
    let checked = result.is_ok().then_some(&info);
    warnings.extend(naming::warnings(&sources, &program, checked));
    warnings.extend(safety::warnings(&sources, &program));
    allow::apply(&program, &sources, &mut warnings);
    // The libraries' are their own business: what a program doesn't use
    // of one isn't dead, for one. So are the dependencies', which checking
    // each on its own shows. The program's `t.toml` is its own.
    warnings.retain(|w| sources.in_program(w.span.start));
    let mut result = result;
    // Messages name the prelude's traits as programs do.
    let shorten = |t: &str| info.traits.shorten(t);
    for w in &mut warnings {
        w.map_text(&shorten);
    }
    if let Err(e) = &mut result {
        e.map_text(&shorten);
    }
    let info = Arc::new(info);
    let result = result.map(|()| CheckedProgram {
        program,
        info: info.clone(),
    });
    Checked {
        sources,
        warnings,
        info,
        result,
    }
}

#[derive(Clone)]
pub struct RunResult {
    /// The program's files, as far as they were read.
    pub sources: Sources,
    /// Warnings from checking the program.
    pub warnings: Vec<Warning>,
    /// The lines the program wrote to its output, including those before
    /// an error, when its streams were in memory: nothing for the
    /// process's own.
    pub output: Vec<String>,
    /// The lines it wrote to where errors go, the same way.
    pub errors: Vec<String>,
    /// The value `main` returned, or the error that stopped the program.
    pub result: Result<Value, Error>,
    /// Did it stop by panicking: not by running out of memory or of
    /// stack, nor by giving back an `Err`? `t` then fails with 101.
    pub panicked: bool,
}

impl RunResult {
    /// Did the program fail: stop with an error, or give back an `Err` from
    /// `main`, which it reported?
    pub fn failed(&self) -> bool {
        match &self.result {
            Ok(Value::Variant(_, variant, _)) => variant == "Err",
            Ok(_) => false,
            Err(_) => true,
        }
    }
}

/// Checks and then interprets a program of one file, as `check_source`,
/// with no input.
pub fn run_source(src: &str) -> RunResult {
    run_files(
        Path::new("main.t"),
        src,
        Some(Vec::new()),
        Some(interp::Captured::default()),
    )
}

/// Checks and then interprets the program that starts at `path`, whose
/// text is `src`, reading the files of its modules from the file system,
/// or from `files`, by their paths, if it's given. Its streams are the
/// process's, or if `term` is given, in memory: it reads `term`'s input,
/// is given its arguments and environment, and what it writes is in the
/// result.
pub fn run_files(
    path: &Path,
    src: &str,
    files: Option<Vec<(PathBuf, String)>>,
    term: Option<interp::Captured>,
) -> RunResult {
    run(
        path,
        src,
        files,
        term.map(Ok).unwrap_or(Err(Vec::new())),
        |_, _| {},
    )
}

/// Checks the program that starts at `path`, whose text is `src`, reading
/// the files of its modules from the file system, and then, if it checked,
/// interprets it with the process's own streams and environment, and the
/// arguments `args` after its path, as `t` does. Before it runs, `checked`
/// is given its files and warnings, to show them.
pub fn run_process(
    path: &Path,
    src: &str,
    args: Vec<std::ffi::OsString>,
    checked: fn(&Sources, &[Warning]),
) -> RunResult {
    run(path, src, None, Err(args), checked)
}

/// `run_files`, telling `checked` what checking found before it runs: in
/// memory, `Ok`, or with the process's streams and these arguments, `Err`.
fn run(
    path: &Path,
    src: &str,
    files: Option<Vec<(PathBuf, String)>>,
    term: Result<interp::Captured, Vec<std::ffi::OsString>>,
    checked: fn(&Sources, &[Warning]),
) -> RunResult {
    let (path, src) = (path.to_owned(), src.to_owned());
    on_big_stack(move || {
        let read = |p: &Path| match &files {
            None => load::read_file(p),
            Some(files) => match files.iter().find(|(f, _)| f == p) {
                Some((_, text)) => Ok(text.clone()),
                None => no_files(p),
            },
        };
        let Checked {
            sources,
            warnings,
            result,
            ..
        } = check_files(&path, &src, &read);
        checked(&sources, &warnings);
        let result = result.and_then(|program| match program.program().is_library() {
            true => {
                let start = sources.first().start;
                Err(Error::ty(
                    error::Span::new(start, start),
                    "this is a library, which starts in `lib.t` and has no `main` to run; \
                         `t check` checks it",
                ))
            }
            false => Ok(program),
        });
        match result.map(|program| program.lower()) {
            Err(e) => RunResult {
                sources,
                warnings,
                output: Vec::new(),
                errors: Vec::new(),
                result: Err(e),
                panicked: false,
            },
            Ok(program) => match term {
                Err(args) => {
                    let mut term = Process::new(path.as_os_str().to_owned(), args);
                    let interp::Ended { result, panicked } =
                        interp::run_entry(&program, "main", &mut term);
                    // A write may have reached only the buffer. Flush
                    // even after an error, keeping that error if there
                    // was one, and otherwise report a failed flush.
                    let flushed = std::io::Write::flush(&mut term.out);
                    let result = result.and_then(|value| {
                        flushed.map(|()| value).map_err(|e| {
                            let main = program
                                .program()
                                .fns
                                .iter()
                                .find(|f| f.name.name == "main")
                                .expect("a checked program has a main function");
                            Error::runtime(main.name.span, format!("cannot flush stdout: {e}"))
                        })
                    });
                    RunResult {
                        sources,
                        warnings,
                        output: Vec::new(),
                        errors: Vec::new(),
                        result,
                        panicked,
                    }
                }
                Ok(mut term) => {
                    let interp::Ended { result, panicked } =
                        interp::run_entry(&program, "main", &mut term);
                    RunResult {
                        sources,
                        warnings,
                        output: lines(&term.out),
                        errors: lines(&term.err),
                        result,
                        panicked,
                    }
                }
            },
        }
    })
}

/// What `t build` made of a program: its files and warnings, as for a
/// run, and its C, or the error that stopped it.
pub struct Built {
    pub sources: Sources,
    pub warnings: Vec<Warning>,
    pub result: Result<String, Error>,
}

/// Checks the program that starts at `path`, whose text is `src`, reading
/// the files of its modules with `read`, and writes it as C (see
/// `cgen.rs`).
pub fn build_c(path: &Path, src: &str, read: load::Read, options: cgen::Options) -> Built {
    {
        let Checked {
            sources,
            warnings,
            result,
            ..
        } = check_files_for(path, src, read, options.target);
        let result = result.and_then(|program| match program.program().is_library() {
            true => {
                let start = sources.first().start;
                Err(Error::ty(
                    error::Span::new(start, start),
                    "this is a library, which starts in `lib.t` and has no `main` to run or \
                     build; `t check` checks it",
                ))
            }
            false => Ok(cgen::emit(&program.lower(), &sources, options)),
        });
        Built {
            sources,
            warnings,
            result,
        }
    }
}

/// What `t test` found: the program's files and warnings, as for a run,
/// and how each of its tests went, in order, or the error that kept them
/// from running.
pub struct TestRun {
    pub sources: Sources,
    pub warnings: Vec<Warning>,
    pub result: Result<Vec<TestOutcome>, Error>,
}

/// How one test went.
pub struct TestOutcome {
    /// Its name: its module's path, then the function's.
    pub name: String,
    /// Why it failed, or nothing if it passed.
    pub failure: Option<Failure>,
}

/// Why a test failed: the interpreter's error, or what a compiled test
/// said where errors go, which is what the interpreter's error says
/// (docs/33-c-backend.md), or how it ended if it said nothing.
#[derive(Debug, Clone)]
pub enum Failure {
    Error(Error),
    Said(String),
}

impl Failure {
    /// It as it's shown, with the program's files `sources`.
    pub fn render_in(&self, sources: &Sources) -> String {
        match self {
            Failure::Error(e) => e.render_in(sources),
            Failure::Said(text) => text.clone(),
        }
    }
}

impl TestRun {
    /// Did a test fail, or did the tests not run?
    pub fn failed(&self) -> bool {
        match &self.result {
            Ok(tests) => tests.iter().any(|t| t.failure.is_some()),
            Err(_) => true,
        }
    }
}

/// Checks the program that starts at `path`, whose text is `src`, reading
/// its files with `read`, and runs each of its own tests whose name has
/// `filter` in it, in order, each as a program of its own: from nothing,
/// with no input, and with nothing shared (see docs/31-tests.md).
pub fn test_files(path: &Path, src: &str, read: load::Read, filter: Option<&str>) -> TestRun {
    let Checked {
        sources,
        warnings,
        info,
        result,
    } = check_files(path, src, read);
    let program = match result {
        Ok(program) => program.lower(),
        Err(e) => {
            return TestRun {
                sources,
                warnings,
                result: Err(e),
            };
        }
    };
    let program = Arc::new(program);
    let tests = (info.tests.iter()).filter(|t| filter.is_none_or(|f| t.symbol.contains(f)));
    let outcomes = tests
        .map(|test| {
            let (program, symbol) = (program.clone(), test.symbol.clone());
            let ended = on_big_stack(move || {
                let mut term = interp::Captured::default();
                let ended = interp::run_entry(&program, &symbol, &mut term);
                (ended, lines(&term.err))
            });
            let (ended, errors) = ended;
            let failure = match (ended.result, test.panics) {
                (Err(_), true) if ended.panicked => None,
                (Err(e), _) => Some(Failure::Error(e)),
                (Ok(Value::Variant(_, variant, _)), _) if variant == "Err" => {
                    let why = errors.join("\n");
                    let why = why.strip_prefix("error: ").unwrap_or(&why);
                    Some(Failure::Error(Error::runtime(
                        test.span,
                        format!("gave back an error: {why}"),
                    )))
                }
                (Ok(_), true) => Some(Failure::Error(Error::runtime(
                    test.span,
                    "didn't panic, which this test passes only by doing",
                ))),
                (Ok(_), false) => None,
            };
            TestOutcome {
                name: test.symbol.clone(),
                failure,
            }
        })
        .collect();
    TestRun {
        sources,
        warnings,
        result: Ok(outcomes),
    }
}

/// `test_files`, compiled: checks the program, writes one C program that
/// runs any of its tests (`cgen::emit_tests`), compiles it with the C
/// compiler, and runs it once for each test whose name has `filter` in
/// it, in order, as a process of its own: from nothing, with no input,
/// its output thrown away, and with nothing shared (docs/44-runs-and-holes.md).
/// Its status says how it went: 0 passed, 101 panicked, and anything
/// else failed some other way. What it wrote where errors go is why it
/// failed, as the interpreter would say it.
pub fn test_native(
    path: &Path,
    src: &str,
    read: load::Read,
    filter: Option<&str>,
) -> Result<TestRun, cgen::CompileError> {
    let Checked {
        sources,
        warnings,
        info,
        result,
    } = check_files(path, src, read);
    let program = match result {
        Ok(program) => program.lower(),
        Err(e) => {
            return Ok(TestRun {
                sources,
                warnings,
                result: Err(e),
            });
        }
    };
    let tests: Vec<_> = (info.tests.iter())
        .filter(|t| filter.is_none_or(|f| t.symbol.contains(f)))
        .collect();
    if tests.is_empty() {
        return Ok(TestRun {
            sources,
            warnings,
            result: Ok(Vec::new()),
        });
    }
    let symbols: Vec<String> = tests.iter().map(|t| t.symbol.clone()).collect();
    let options = cgen::Options {
        lines: true,
        target: Target::Hosted,
    };
    let c = cgen::emit_tests(&program, &sources, options, &symbols);
    let dir = scratch_dir("test");
    let exe = dir.join("tests");
    let result = cgen::compile(&c, &exe, Target::Hosted).map(|()| {
        (tests.iter().enumerate())
            .map(|(i, test)| TestOutcome {
                name: test.symbol.clone(),
                failure: run_test(&exe, i, test.span, test.panics),
            })
            .collect()
    });
    let _ = std::fs::remove_dir_all(&dir);
    Ok(TestRun {
        sources,
        warnings,
        result: Ok(result?),
    })
}

/// Runs the `i`th test of the compiled tests `exe`, whose name is at
/// `span`, and which passes only by panicking if `panics`, and says why it
/// failed, if it did.
fn run_test(exe: &Path, i: usize, span: error::Span, panics: bool) -> Option<Failure> {
    let output = std::process::Command::new(exe)
        .arg(i.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output();
    let output = match output {
        Ok(output) => output,
        Err(e) => return Some(Failure::Said(format!("couldn't be run: {e}"))),
    };
    let said = String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_string();
    let code = output.status.code();
    match (code, panics) {
        (Some(101), true) => None,
        (Some(0), false) => None,
        (Some(0), true) => Some(Failure::Error(Error::runtime(
            span,
            "didn't panic, which this test passes only by doing",
        ))),
        // `main`'s error, reported as `t run` reports it.
        (Some(1), _) if said.starts_with("error: ") => {
            let why = said.strip_prefix("error: ").unwrap_or(&said);
            Some(Failure::Error(Error::runtime(
                span,
                format!("gave back an error: {why}"),
            )))
        }
        _ if !said.is_empty() => Some(Failure::Said(said)),
        _ => Some(Failure::Said(format!("ended with {}", output.status))),
    }
}

/// A new directory of its own in the system's, for a compiled program,
/// named for `what` it's for: whoever makes it removes it.
pub fn scratch_dir(what: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "t-lang-{what}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// The stack each thread of the interpreter's runs on (see `on_big_stack`).
pub(crate) const INTERP_STACK_SIZE: usize = 1024 * 1024 * 1024;

/// `f`'s result, worked out on a thread of its own with a big stack: the
/// interpreter recurses several times per nested call, so give it enough
/// to reach `MAX_CALL_DEPTH` even in debug builds, where frames are huge.
/// It's only reserved address space; untouched pages cost nothing.
fn on_big_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .stack_size(INTERP_STACK_SIZE)
        .spawn(f)
        .expect("failed to spawn interpreter thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// The lines of `bytes`, without their line breaks: the last needn't end in
/// one. Bytes that aren't UTF-8 are shown as `\u{FFFD}`.
fn lines(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_suffix('\n').unwrap_or(&text);
    match text.is_empty() && bytes.is_empty() {
        true => Vec::new(),
        false => text.split('\n').map(str::to_string).collect(),
    }
}

/// The process's own streams, for a program `t` runs, and the arguments
/// and environment it's given. Output is buffered, and written out before
/// each read of input, so that a prompt is seen before it's answered, and
/// before anything is written where errors go.
struct Process {
    input: std::io::BufReader<std::io::Stdin>,
    out: std::io::BufWriter<std::io::Stdout>,
    err: std::io::Stderr,
    args: Vec<String>,
    env: Vec<(String, String)>,
    /// What's wrong with the arguments or the environment, if one isn't
    /// UTF-8.
    not_unicode: Option<String>,
    /// The files open to be read, by number, each through a buffer, as a
    /// compiled program's are; `None` once closed.
    files: Vec<Option<std::io::BufReader<std::fs::File>>>,
}

impl Process {
    /// The process's streams and environment, and arguments `args` after
    /// the program's name `program`.
    fn new(program: std::ffi::OsString, args: Vec<std::ffi::OsString>) -> Process {
        let mut not_unicode = None;
        let mut text = |s: std::ffi::OsString, what: &dyn Fn() -> String| match s.into_string() {
            Ok(s) => s,
            Err(s) => {
                not_unicode.get_or_insert_with(what);
                s.to_string_lossy().into_owned()
            }
        };
        let program = text(program, &|| "the program's name isn't UTF-8".to_string());
        let args: Vec<String> = std::iter::once(program)
            .chain(
                (args.into_iter().enumerate())
                    .map(|(i, a)| text(a, &|| format!("argument {i} isn't UTF-8"))),
            )
            .collect();
        let env = std::env::vars_os()
            .map(|(name, value)| {
                let shown = name.to_string_lossy().into_owned();
                let what = || format!("environment variable `{shown}` isn't UTF-8");
                (text(name, &what), text(value, &what))
            })
            .collect();
        Process {
            input: std::io::BufReader::new(std::io::stdin()),
            out: std::io::BufWriter::new(std::io::stdout()),
            err: std::io::stderr(),
            args,
            env,
            not_unicode,
            files: Vec::new(),
        }
    }
}

impl interp::Terminal for Process {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::io::{Read, Write};
        self.out.flush()?;
        self.input.read(buf)
    }

    fn write(&mut self, err: bool, buf: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        match err {
            false => self.out.write_all(buf),
            // Whether the output can still be written or not, errors can.
            true => {
                let _ = self.out.flush();
                self.err.write_all(buf)
            }
        }
    }

    fn flush(&mut self, err: bool) -> std::io::Result<()> {
        use std::io::Write;
        match err {
            false => self.out.flush(),
            true => self.err.flush(),
        }
    }

    fn is_terminal(&self, stream: Option<bool>) -> bool {
        use std::io::IsTerminal;
        match stream {
            None => std::io::stdin().is_terminal(),
            Some(false) => std::io::stdout().is_terminal(),
            Some(true) => std::io::stderr().is_terminal(),
        }
    }

    fn args(&self) -> &[String] {
        &self.args
    }

    fn env(&self) -> &[(String, String)] {
        &self.env
    }

    fn not_unicode(&self) -> Option<&str> {
        self.not_unicode.as_deref()
    }

    fn open(&mut self, path: &str) -> std::io::Result<usize> {
        let file = std::fs::File::open(path)?;
        self.files.push(Some(std::io::BufReader::new(file)));
        Ok(self.files.len() - 1)
    }

    fn read_file(&mut self, file: usize, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::io::Read;
        match self.files.get_mut(file) {
            Some(Some(f)) => loop {
                match f.read(buf) {
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    r => break r,
                }
            },
            _ => panic!("a read of file {file}, which isn't open"),
        }
    }

    fn close(&mut self, file: usize) {
        match self.files.get_mut(file) {
            Some(f @ Some(_)) => *f = None,
            _ => panic!("file {file} closed, which isn't open"),
        }
    }

    fn write_file(&mut self, path: &str, bytes: &[u8], append: bool) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(path)?;
        file.write_all(bytes)
    }

    fn remove(&mut self, path: &str) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }
}

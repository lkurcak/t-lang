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
pub mod check;
mod doc;
pub mod error;
pub mod exhaust;
pub mod fmt;
pub mod interp;
pub mod lexer;
pub mod lift;
pub mod load;
pub mod lsp;
pub mod mono;
mod naming;
pub mod parser;
mod pretty;
pub mod rename;
mod safety;
pub mod source;
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
    let mut warnings = Vec::new();
    let (sources, program) = load::load(path, src, read);
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
    // each on its own shows.
    warnings.retain(|w| !sources.in_others(w.span.start));
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
    run_files(Path::new("main.t"), src, Some(Vec::new()), Some(Vec::new()))
}

/// Checks and then interprets the program that starts at `path`, whose
/// text is `src`, reading the files of its modules from the file system,
/// or from `files`, by their paths, if it's given. Its streams are the
/// process's, or if `input` is given, in memory: it reads `input`, and
/// what it writes is in the result.
pub fn run_files(
    path: &Path,
    src: &str,
    files: Option<Vec<(PathBuf, String)>>,
    input: Option<Vec<u8>>,
) -> RunResult {
    run(path, src, files, input, |_, _| {})
}

/// Checks the program that starts at `path`, whose text is `src`, reading
/// the files of its modules from the file system, and then, if it checked,
/// interprets it with the process's own streams, as `t` does. Before it
/// runs, `checked` is given its files and warnings, to show them.
pub fn run_process(path: &Path, src: &str, checked: fn(&Sources, &[Warning])) -> RunResult {
    run(path, src, None, None, checked)
}

/// `run_files`, telling `checked` what checking found before it runs.
fn run(
    path: &Path,
    src: &str,
    files: Option<Vec<(PathBuf, String)>>,
    input: Option<Vec<u8>>,
    checked: fn(&Sources, &[Warning]),
) -> RunResult {
    // The interpreter recurses several times per nested call, so give it a
    // big stack: enough to reach `MAX_CALL_DEPTH` even in debug builds, where
    // frames are huge. It's only reserved address space; untouched pages cost
    // nothing.
    const STACK_SIZE: usize = 1024 * 1024 * 1024;
    let (path, src) = (path.to_owned(), src.to_owned());
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
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
                },
                Ok(program) => match input {
                    None => {
                        let mut term = Process::default();
                        let result = interp::run(&program, &mut term);
                        let result = result.map_err(|e| e.blamed_caller(&sources));
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
                        }
                    }
                    Some(input) => {
                        let mut term = interp::Captured {
                            input: input.into(),
                            ..interp::Captured::default()
                        };
                        let result = interp::run(&program, &mut term);
                        let result = result.map_err(|e| e.blamed_caller(&sources));
                        RunResult {
                            sources,
                            warnings,
                            output: lines(&term.out),
                            errors: lines(&term.err),
                            result,
                        }
                    }
                },
            }
        })
        .expect("failed to spawn interpreter thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
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
    pub failure: Option<Error>,
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
                (Err(e), _) => Some(e.blamed_caller(&sources)),
                (Ok(Value::Variant(_, variant, _)), _) if variant == "Err" => {
                    let why = errors.join("\n");
                    let why = why.strip_prefix("error: ").unwrap_or(&why);
                    Some(Error::runtime(test.span, format!("gave back an error: {why}")))
                }
                (Ok(_), true) => Some(Error::runtime(
                    test.span,
                    "didn't panic, which this test passes only by doing",
                )),
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

/// `f`'s result, worked out on a thread of its own with a big stack: the
/// interpreter recurses several times per nested call, so give it enough
/// to reach `MAX_CALL_DEPTH` even in debug builds, where frames are huge.
/// It's only reserved address space; untouched pages cost nothing.
fn on_big_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    const STACK_SIZE: usize = 1024 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
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

/// The process's own streams, for a program `t` runs. Output is buffered,
/// and written out before each read of input, so that a prompt is seen
/// before it's answered, and before anything is written where errors go.
struct Process {
    input: std::io::BufReader<std::io::Stdin>,
    out: std::io::BufWriter<std::io::Stdout>,
    err: std::io::Stderr,
}

impl Default for Process {
    fn default() -> Process {
        Process {
            input: std::io::BufReader::new(std::io::stdin()),
            out: std::io::BufWriter::new(std::io::stdout()),
            err: std::io::stderr(),
        }
    }
}

impl interp::Terminal for Process {
    fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
        use std::io::{Read, Write};
        self.out.flush()?;
        let mut b = [0];
        loop {
            match self.input.read(&mut b) {
                Ok(0) => return Ok(None),
                Ok(_) => return Ok(Some(b[0])),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
    }

    fn write_byte(&mut self, err: bool, b: u8) -> std::io::Result<()> {
        use std::io::Write;
        match err {
            false => self.out.write_all(&[b]),
            // Whether the output can still be written or not, errors can.
            true => {
                let _ = self.out.flush();
                self.err.write_all(&[b])
            }
        }
    }
}

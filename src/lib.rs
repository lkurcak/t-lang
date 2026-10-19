//! The T compiler. Pipeline: source → `lexer` → `parser` → `check` (and
//! `borrowck`, once each function's types are known) → `lift` → `mono` →
//! `interp`.

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
pub mod lsp;
pub mod mono;
mod naming;
pub mod parser;
mod pretty;
pub mod rename;
pub mod traits;
mod trivia;
pub mod types;

use error::{Error, Warning};
use interp::Value;

pub struct Checked {
    /// Warnings found, including those before an error.
    pub warnings: Vec<Warning>,
    /// What the checker learned about the program, as far as it got.
    pub info: check::Info,
    /// The checked program, lifted (so with no nested functions left) and
    /// monomorphised (so with no generics left), or the first error.
    pub result: Result<ast::Program, Error>,
}

/// Lexes, parses, type-checks, lifts and monomorphises a program.
pub fn check_source(src: &str) -> Checked {
    let mut warnings = Vec::new();
    let mut program = match lexer::lex(src).and_then(parser::parse) {
        Ok(program) => program,
        Err(e) => {
            return Checked {
                warnings,
                info: check::Info::default(),
                result: Err(e),
            };
        }
    };
    let (info, result) = check::check(&mut program, &mut warnings);
    // Here rather than in `check`, since the fixes check renamed programs.
    let checked = result.is_ok().then_some(&info);
    warnings.extend(naming::warnings(src, &program, checked));
    allow::apply(&program, &mut warnings);
    let result = result.map(|()| mono::monomorphise(&lift::lift(&program, &info), &info));
    Checked {
        warnings,
        info,
        result,
    }
}

#[derive(Clone)]
pub struct RunResult {
    /// Warnings from checking the program.
    pub warnings: Vec<Warning>,
    /// Lines printed with `print`, including those before an error.
    pub output: Vec<String>,
    /// The value `main` returned, or the error that stopped the program.
    pub result: Result<Value, Error>,
}

/// Checks and then interprets a program.
pub fn run_source(src: &str) -> RunResult {
    // The interpreter recurses several times per nested call, so give it a
    // big stack: enough to reach `MAX_CALL_DEPTH` even in debug builds, where
    // frames are huge. It's only reserved address space; untouched pages cost
    // nothing.
    const STACK_SIZE: usize = 1024 * 1024 * 1024;
    let src = src.to_owned();
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let Checked {
                warnings, result, ..
            } = check_source(&src);
            match result {
                Err(e) => RunResult {
                    warnings,
                    output: Vec::new(),
                    result: Err(e),
                },
                Ok(program) => {
                    let mut output = Vec::new();
                    let result = interp::run(&program, &mut output);
                    RunResult {
                        warnings,
                        output,
                        result,
                    }
                }
            }
        })
        .expect("failed to spawn interpreter thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

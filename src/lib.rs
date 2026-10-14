//! The T compiler. Pipeline: source → `lexer` → `parser` → `check` → `interp`.

pub mod ast;
pub mod check;
pub mod error;
pub mod exhaust;
pub mod interp;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod types;

use error::{Error, Warning};
use interp::Value;

pub struct Checked {
    /// Warnings found, including those before an error.
    pub warnings: Vec<Warning>,
    /// The checked program, or the first error.
    pub result: Result<ast::Program, Error>,
}

/// Lexes, parses and type-checks a program.
pub fn check_source(src: &str) -> Checked {
    let mut warnings = Vec::new();
    let result = (|| {
        let tokens = lexer::lex(src)?;
        let program = parser::parse(tokens)?;
        check::check(&program, &mut warnings)?;
        Ok(program)
    })();
    Checked { warnings, result }
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
            let Checked { warnings, result } = check_source(&src);
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

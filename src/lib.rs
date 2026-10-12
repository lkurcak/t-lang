//! The T compiler. Pipeline: source → `lexer` → `parser` → `check` → `interp`.

pub mod ast;
pub mod check;
pub mod error;
pub mod interp;
pub mod lexer;
pub mod parser;
pub mod types;

use error::Error;
use interp::Value;

/// Lexes, parses and type-checks a program.
pub fn check_source(src: &str) -> Result<ast::Program, Error> {
    let tokens = lexer::lex(src)?;
    let program = parser::parse(tokens)?;
    check::check(&program)?;
    Ok(program)
}

pub struct RunResult {
    /// Lines printed with `print`, including those before an error.
    pub output: Vec<String>,
    /// The value `main` returned, or the error that stopped the program.
    pub result: Result<Value, Error>,
}

/// Checks and then interprets a program.
pub fn run_source(src: &str) -> RunResult {
    // The interpreter recurses once per nested call, so give it a big stack.
    const STACK_SIZE: usize = 256 * 1024 * 1024;
    let src = src.to_owned();
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || match check_source(&src) {
            Err(e) => RunResult { output: Vec::new(), result: Err(e) },
            Ok(program) => {
                let mut output = Vec::new();
                let result = interp::run(&program, &mut output);
                RunResult { output, result }
            }
        })
        .expect("failed to spawn interpreter thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

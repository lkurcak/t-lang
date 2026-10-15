//! Runs a program and compares it against the expectations written in its
//! comments, for `tests/t/*.t` (`golden.rs`) and the T code in the docs
//! (`docs.rs`):
//!
//! - `// expect: <line>` — anywhere on a line, one per expected output line, in
//!   order. The output is everything `print`ed, then `main`'s return value
//!   unless it is `()`.
//! - `// error: <text>` — the program must fail (to parse, to type-check, or at
//!   runtime) with an error whose rendered message contains `<text>`. Any
//!   `expect` lines must then match what was printed before the error.
//! - `// warning: <text>` — one per warning, in order, each contained in the
//!   rendered warning. A warning without one is a failure.
//!
//! Fixes are checked too. Applying warnings' fixes one after another must
//! leave a program that does exactly the same thing, and run out of fixes
//! (one fix can lead to another: narrowing `0..10 | 5..20` gives
//! `0..10 | 10..20`, which can then be merged). Applying an error's first fix
//! must get rid of that error.

use t_lang::RunResult;
use t_lang::interp::Value;

/// What the comments in a program say it should do.
pub struct Expectations {
    pub output: Vec<String>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

impl Expectations {
    pub fn of(src: &str) -> Expectations {
        let directive = |line: &str, marker: &str| -> Option<String> {
            line.find(marker)
                .map(|i| line[i + marker.len()..].trim().to_string())
        };
        Expectations {
            output: src
                .lines()
                .filter_map(|l| directive(l, "// expect: "))
                .collect(),
            error: src.lines().find_map(|l| directive(l, "// error: ")),
            warnings: src
                .lines()
                .filter_map(|l| directive(l, "// warning: "))
                .collect(),
        }
    }
}

/// Runs `src`, named `name` in messages, and checks that it does what its
/// comments say.
pub fn check(name: &str, src: &str) -> Result<(), String> {
    let want = Expectations::of(src);
    let run = t_lang::run_source(src);
    let warnings: Vec<String> = run.warnings.iter().map(|w| w.render(name, src)).collect();
    let matches = warnings.len() == want.warnings.len()
        && warnings
            .iter()
            .zip(&want.warnings)
            .all(|(w, want)| w.contains(want));
    if !matches {
        return Err(format!(
            "expected warnings {:?}, got\n{}",
            want.warnings,
            warnings.join("\n\n")
        ));
    }
    check_fixes(src, &run)?;

    let mut actual = run.output;
    match (run.result, want.error) {
        (Ok(value), None) => {
            if value != Value::Unit {
                actual.push(value.to_string());
            }
        }
        (Ok(_), Some(want)) => {
            return Err(format!(
                "expected an error containing {want:?}, but it ran fine"
            ));
        }
        (Err(e), None) => return Err(format!("unexpected error\n{}", e.render(name, src))),
        (Err(e), Some(want)) => {
            let rendered = e.render(name, src);
            if !rendered.contains(&want) {
                return Err(format!(
                    "expected an error containing {want:?}, got\n{rendered}"
                ));
            }
        }
    }
    if actual != want.output {
        return Err(format!("expected output {:?}, got {actual:?}", want.output));
    }
    Ok(())
}

/// Applies fixes one at a time: for each warning in turn, then for the error.
fn check_fixes(src: &str, run: &RunResult) -> Result<(), String> {
    let (mut src, mut run) = (src.to_string(), run.clone());
    let mut rounds = 0;
    while let Some(fix) = run.warnings.iter().find_map(|w| w.fixes.first()) {
        rounds += 1;
        if rounds > 10 {
            return Err(format!("fixes never stop coming:\n{src}"));
        }
        let fixed = fix.apply(&src);
        let after = t_lang::run_source(&fixed);
        if !same_behaviour(&after, &run) {
            return Err(format!(
                "fix {:?} changed what the program does:\n{fixed}",
                fix.title
            ));
        }
        (src, run) = (fixed, after);
    }
    if let Err(e) = &run.result
        && let Some(fix) = e.fixes.first()
    {
        let fixed = fix.apply(&src);
        if let Err(after) = t_lang::run_source(&fixed).result
            && after.message == e.message
            && after.span == e.span
        {
            return Err(format!(
                "fix {:?} didn't fix the error:\n{fixed}",
                fix.title
            ));
        }
    }
    Ok(())
}

/// Do two runs print the same, and end the same way?
pub fn same_behaviour(a: &RunResult, b: &RunResult) -> bool {
    a.output == b.output
        && match (&a.result, &b.result) {
            (Ok(a), Ok(b)) => a == b,
            (Err(a), Err(b)) => a.message == b.message,
            _ => false,
        }
}

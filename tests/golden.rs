//! Runs every `tests/t/*.t` program and compares it against the expectations
//! written in its comments:
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
//!
//! A `tests/t/<name>.mono` file next to a test holds what `t mono` prints
//! for it: the program after monomorphisation. Run with `BLESS=1` to rewrite
//! those files from what it prints now.

use std::fs;
use std::path::{Path, PathBuf};
use t_lang::RunResult;
use t_lang::interp::Value;

/// Every `tests/t/*.t` file, in order.
fn programs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "t"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no tests found in {}", dir.display());
    paths
}

#[test]
fn golden() {
    let paths = programs();

    let mut failures = Vec::new();
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let src = fs::read_to_string(path).unwrap();
        let directive = |line: &'_ str, marker: &str| -> Option<String> {
            line.find(marker)
                .map(|i| line[i + marker.len()..].trim().to_string())
        };
        let expected: Vec<String> = src
            .lines()
            .filter_map(|l| directive(l, "// expect: "))
            .collect();
        let expected_error = src.lines().find_map(|l| directive(l, "// error: "));
        let expected_warnings: Vec<String> = src
            .lines()
            .filter_map(|l| directive(l, "// warning: "))
            .collect();
        if expected.is_empty() && expected_error.is_none() && expected_warnings.is_empty() {
            failures.push(format!(
                "{name}: has no `// expect:`, `// error:` or `// warning:` lines"
            ));
            continue;
        }

        let run = t_lang::run_source(&src);
        let warnings: Vec<String> = run.warnings.iter().map(|w| w.render(&name, &src)).collect();
        let matches = warnings.len() == expected_warnings.len()
            && warnings
                .iter()
                .zip(&expected_warnings)
                .all(|(w, want)| w.contains(want));
        if !matches {
            failures.push(format!(
                "{name}: expected warnings {expected_warnings:?}, got\n{}",
                warnings.join("\n\n")
            ));
            continue;
        }
        if let Err(failure) = check_fixes(&src, &run) {
            failures.push(format!("{name}: {failure}"));
            continue;
        }

        let mut actual = run.output;
        match (run.result, expected_error) {
            (Ok(value), None) => {
                if value != Value::Unit {
                    actual.push(value.to_string());
                }
            }
            (Ok(_), Some(want)) => {
                failures.push(format!(
                    "{name}: expected an error containing {want:?}, but it ran fine"
                ));
                continue;
            }
            (Err(e), None) => {
                failures.push(format!(
                    "{name}: unexpected error\n{}",
                    e.render(&name, &src)
                ));
                continue;
            }
            (Err(e), Some(want)) => {
                let rendered = e.render(&name, &src);
                if !rendered.contains(&want) {
                    failures.push(format!(
                        "{name}: expected an error containing {want:?}, got\n{rendered}"
                    ));
                    continue;
                }
            }
        }
        if actual != expected {
            failures.push(format!(
                "{name}: expected output {expected:?}, got {actual:?}"
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} of {} golden tests failed:\n\n{}\n",
            failures.len(),
            paths.len(),
            failures.join("\n\n")
        );
    }
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
        let same = after.output == run.output
            && match (&after.result, &run.result) {
                (Ok(a), Ok(b)) => a == b,
                (Err(a), Err(b)) => a.message == b.message,
                _ => false,
            };
        if !same {
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

/// What monomorphisation makes of each program with a `.mono` file.
#[test]
fn mono() {
    let bless = std::env::var_os("BLESS").is_some();
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in programs() {
        let mono = path.with_extension("mono");
        if !mono.exists() {
            continue;
        }
        checked += 1;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let src = fs::read_to_string(&path).unwrap();
        let actual = match t_lang::check_source(&src).result {
            Ok(program) => program.to_string(),
            Err(e) => {
                failures.push(format!("{name}: {}", e.render(&name, &src)));
                continue;
            }
        };
        if bless {
            fs::write(&mono, &actual).unwrap();
        } else if fs::read_to_string(&mono).unwrap() != actual {
            failures.push(format!(
                "{name}: `t mono` printed something else (run with BLESS=1 to update \
                 {}):\n{actual}",
                mono.display()
            ));
        }
    }
    assert!(checked > 0, "no `.mono` files");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// A program without generics prints as itself: parsing what `t mono`
/// prints gives a program that prints the same, and does the same.
#[test]
fn printing_round_trips() {
    let mut failures = Vec::new();
    for path in programs() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let src = fs::read_to_string(&path).unwrap();
        let Ok(program) = t_lang::check_source(&src).result else {
            continue;
        };
        let printed = program.to_string();
        if printed.contains("::<") {
            continue;
        }
        let reprinted = match t_lang::check_source(&printed).result {
            Ok(p) => p.to_string(),
            Err(e) => {
                failures.push(format!("{name}: {}", e.render(&name, &printed)));
                continue;
            }
        };
        if reprinted != printed {
            failures.push(format!("{name}: printed\n{printed}\nthen\n{reprinted}"));
            continue;
        }
        let (before, after) = (t_lang::run_source(&src), t_lang::run_source(&printed));
        let same = before.output == after.output
            && match (&before.result, &after.result) {
                (Ok(a), Ok(b)) => a == b,
                (Err(a), Err(b)) => a.message == b.message,
                _ => false,
            };
        if !same {
            failures.push(format!(
                "{name}: does something else once printed:\n{printed}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

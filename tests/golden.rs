//! Runs every `tests/t/*.t` program and compares it against the expectations
//! written in its comments: `// expect:`, `// error:` and `// warning:` (see
//! `shared/expect.rs`). Every test must have at least one.
//!
//! A `tests/t/<name>.mono` file next to a test holds what `t mono` prints
//! for it: the program after monomorphisation. Run with `BLESS=1` to rewrite
//! those files from what it prints now.

#[path = "shared/expect.rs"]
mod expect;

use expect::Expectations;
use std::fs;
use std::path::{Path, PathBuf};

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
        let want = Expectations::of(&src);
        if want.output.is_empty() && want.error.is_none() && want.warnings.is_empty() {
            failures.push(format!(
                "{name}: has no `// expect:`, `// error:` or `// warning:` lines"
            ));
            continue;
        }
        if let Err(failure) = expect::check(&name, &src) {
            failures.push(format!("{name}: {failure}"));
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

/// A program without generics or nested functions prints as itself:
/// parsing what `t mono` prints gives a program that prints the same, and
/// does the same. (The names of the copies of generic things, like
/// `id::<int>`, and of lifted nested functions, like `main::helper`, don't
/// parse.)
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
        if printed.contains("::<") || program.fns.iter().any(|f| f.name.name.contains("::")) {
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
        if !expect::same_behaviour(&before, &after) {
            failures.push(format!(
                "{name}: does something else once printed:\n{printed}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

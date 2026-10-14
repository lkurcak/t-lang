//! Runs every `tests/t/*.t` program and compares it against the expectations
//! written in its comments:
//!
//! - `// expect: <line>` — anywhere on a line, one per expected output line, in
//!   order. The output is everything `print`ed, then `main`'s return value
//!   unless it is `()`.
//! - `// error: <text>` — the program must fail (to parse, to type-check, or at
//!   runtime) with an error whose rendered message contains `<text>`. Any
//!   `expect` lines must then match what was printed before the error.

use std::fs;
use std::path::Path;
use t_lang::interp::Value;

#[test]
fn golden() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "t"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no tests found in {}", dir.display());

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
        if expected.is_empty() && expected_error.is_none() {
            failures.push(format!("{name}: has no `// expect:` or `// error:` lines"));
            continue;
        }

        let run = t_lang::run_source(&src);
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

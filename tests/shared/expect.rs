//! Runs a program and compares it against the expectations written in its
//! comments, for `tests/t/*.t` (`golden.rs`) and the T code in the docs
//! (`docs.rs`):
//!
//! - `// expect: <line>` — anywhere on a line, one per expected output line, in
//!   order: what the program writes to its `stdout`.
//! - `// stderr: <line>` — the same, for what it writes to its `stderr`, as a
//!   `main` that gives back `Err(e)` does.
//! - `// stdin: <line>` — one per line of input the program reads, in order.
//! - `// error: <text>` — the program must fail (to parse, to type-check, or at
//!   runtime) with an error whose rendered message contains `<text>`, not
//!   counting the lines of source it shows, which hold the comment. Any
//!   `expect` lines must then match what was printed before the error.
//! - `// warning: <text>` — one per warning, in order, each contained in the
//!   rendered warning, not counting its lines of source. A warning without
//!   one is a failure.
//! - `// test: <name> ... ok` or `// test: <name> ... FAILED: <text>` — one
//!   per test `t test` runs, in order: it passed, or it failed with a
//!   failure whose rendering contains `<text>`, not counting its lines of
//!   source, and on one line, each run of spaces and line breaks a space.
//!   With these, the program's tests are run too, after the program.
//!
//! Fixes are checked too. Applying warnings' fixes one after another must
//! leave a program that does exactly the same thing, and run out of fixes
//! (one fix can lead to another: narrowing `0..10 | 5..20` gives
//! `0..10 | 10..20`, which can then be merged). Applying an error's first fix
//! must get rid of that error.
//!
//! A program of several files (`mod m;`) has its comments read from all of
//! them, in the order of its sources: the first file, then each module's,
//! as warnings are in.

// Each test binary uses only some of this.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use t_lang::RunResult;

/// A program's files, its first first, by path.
pub type Files = Vec<(PathBuf, String)>;

/// Runs a program of the files `files`.
pub fn run(files: &Files) -> RunResult {
    let (path, src) = &files[0];
    // A library has nothing to run: what checking it says is all.
    if path.file_name().is_some_and(|n| n == "lib.t") {
        let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
            Some((_, text)) => Ok(text.clone()),
            None => t_lang::no_files(p),
        };
        let checked = t_lang::check_files(path, src, &read);
        return RunResult {
            sources: checked.sources,
            warnings: checked.warnings,
            output: Vec::new(),
            errors: Vec::new(),
            result: checked.result.map(|_| t_lang::interp::Value::Unit),
        };
    }
    let texts: Vec<&str> = files.iter().map(|(_, src)| src.as_str()).collect();
    let input = Expectations::of(&texts.join("\n")).input;
    t_lang::run_files(path, src, Some(files.clone()), Some(input))
}

/// What the comments in a program say it should do.
pub struct Expectations {
    pub output: Vec<String>,
    pub errors: Vec<String>,
    /// The input: each `// stdin:` line, and a line break after it.
    pub input: Vec<u8>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub tests: Vec<String>,
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
            errors: src
                .lines()
                .filter_map(|l| directive(l, "// stderr: "))
                .collect(),
            // As written, but for the space after the `:`: an empty line is
            // `// stdin:`.
            input: src
                .lines()
                .filter_map(|l| l.find("// stdin:").map(|i| &l[i + "// stdin:".len()..]))
                .flat_map(|l| l.strip_prefix(' ').unwrap_or(l).bytes().chain(*b"\n"))
                .collect(),
            error: src.lines().find_map(|l| directive(l, "// error: ")),
            warnings: src
                .lines()
                .filter_map(|l| directive(l, "// warning: "))
                .collect(),
            tests: src
                .lines()
                .filter_map(|l| directive(l, "// test: "))
                .collect(),
        }
    }
}

/// Runs `src`, named `name` in messages, and checks that it does what its
/// comments say.
pub fn check(name: &str, src: &str) -> Result<(), String> {
    check_files(&vec![(PathBuf::from(name), src.to_string())])
}

/// Runs the program of `files`, and checks that it does what its comments
/// say.
pub fn check_files(files: &Files) -> Result<(), String> {
    let run = run(files);
    // The expectations of every file, its dependencies' too, but not of
    // its manifests, which aren't T: not what was read, as reading them
    // can be what fails.
    let texts: Vec<&str> = (files.iter())
        .filter(|(p, _)| p.file_name().is_none_or(|n| n != "t.toml"))
        .map(|(_, src)| src.as_str())
        .collect();
    let want = Expectations::of(&texts.join("\n"));
    let warnings = t_lang::error::Warning::render_all_in(&run.warnings, &run.sources);
    let matches = warnings.len() == want.warnings.len()
        && warnings
            .iter()
            .zip(&want.warnings)
            .all(|(w, want)| without_source(w).contains(want));
    if !matches {
        return Err(format!(
            "expected warnings {:?}, got\n{}",
            want.warnings,
            warnings.join("\n\n")
        ));
    }
    check_fixes(files, &run)?;

    let actual = run.output;
    match (run.result, want.error) {
        (Ok(_), None) => {}
        (Ok(_), Some(want)) => {
            return Err(format!(
                "expected an error containing {want:?}, but it ran fine"
            ));
        }
        (Err(e), None) => {
            return Err(format!("unexpected error\n{}", e.render_in(&run.sources)));
        }
        (Err(e), Some(want)) => {
            let rendered = e.render_in(&run.sources);
            if !without_source(&rendered).contains(&want) {
                return Err(format!(
                    "expected an error containing {want:?}, got\n{rendered}"
                ));
            }
        }
    }
    if actual != want.output {
        return Err(format!("expected output {:?}, got {actual:?}", want.output));
    }
    if run.errors != want.errors {
        return Err(format!(
            "expected errors {:?}, got {:?}",
            want.errors, run.errors
        ));
    }
    if !want.tests.is_empty() {
        check_tests(files, &want.tests)?;
    }
    Ok(())
}

/// Runs the tests of the program of `files`, as `t test` does, and checks
/// that each says what `want` says of it.
fn check_tests(files: &Files, want: &[String]) -> Result<(), String> {
    let (path, src) = &files[0];
    let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
        Some((_, text)) => Ok(text.clone()),
        None => t_lang::no_files(p),
    };
    let run = t_lang::test_files(path, src, &read, None);
    let tests = match &run.result {
        Ok(tests) => tests,
        Err(e) => return Err(format!("tests didn't run\n{}", e.render_in(&run.sources))),
    };
    // Each failure on one line, with each run of spaces and line breaks
    // one space, as a `// test:` is.
    let said: Vec<(String, Option<String>)> = (tests.iter())
        .map(|t| {
            let failure = t.failure.as_ref().map(|e| {
                let failure = without_source(&e.render_in(&run.sources));
                failure.split_whitespace().collect::<Vec<_>>().join(" ")
            });
            (t.name.clone(), failure)
        })
        .collect();
    let says = |(name, failure): &(String, Option<String>), want: &String| match failure {
        None => *want == format!("{name} ... ok"),
        Some(failure) => want
            .strip_prefix(&format!("{name} ... FAILED"))
            .is_some_and(|w| {
                w.is_empty() || w.strip_prefix(": ").is_some_and(|w| failure.contains(w))
            }),
    };
    let matches = said.len() == want.len() && said.iter().zip(want).all(|(s, w)| says(s, w));
    let said: Vec<String> = (said.into_iter())
        .map(|(name, failure)| match failure {
            None => format!("{name} ... ok"),
            Some(f) => format!("{name} ... FAILED: {f}"),
        })
        .collect();
    match matches {
        true => Ok(()),
        false => Err(format!("expected tests {want:?}, got\n{}", said.join("\n"))),
    }
}

/// The files of `run`'s program with `fix` made, and those it didn't read
/// as they were.
pub fn fixed(files: &Files, run: &RunResult, fix: &t_lang::error::Fix) -> Files {
    let read = run.sources.program_files().iter().map(|f| f.path.clone());
    let mut out: Files = read.zip(run.sources.edited(&fix.edits)).collect();
    let unread: Files = files
        .iter()
        .filter(|(p, _)| out.iter().all(|(q, _)| p != q))
        .cloned()
        .collect();
    out.extend(unread);
    out
}

/// All of `files`, for messages.
fn shown(files: &Files) -> String {
    match files.as_slice() {
        [(_, src)] => src.clone(),
        _ => files
            .iter()
            .map(|(p, src)| format!("// {}\n{src}", p.display()))
            .collect(),
    }
}

/// A rendered diagnostic without the lines of source it shows, `3 | ..`,
/// and with `/` in the paths it shows.
fn without_source(rendered: &str) -> String {
    let is_source = |l: &str| {
        let (n, rest) = l.split_once(" | ").unwrap_or(("", ""));
        !n.is_empty() && n.trim().chars().all(|c| c.is_ascii_digit()) && !rest.is_empty()
    };
    // Where it is, `--> dir/a/b.t:2:5`, with `/` between a path's parts on
    // every system, as the expectations say them: Windows shows `\`.
    let lines = rendered.lines().filter(|l| !is_source(l)).map(|l| {
        match l.trim_start().starts_with("-->") {
            true => l.replace('\\', "/"),
            false => l.to_string(),
        }
    });
    lines.collect::<Vec<_>>().join("\n")
}

/// Applies fixes one at a time: for each warning in turn, then for the error.
fn check_fixes(files: &Files, run: &RunResult) -> Result<(), String> {
    let (mut files, mut run) = (files.clone(), run.clone());
    let mut rounds = 0;
    while let Some(fix) = run.warnings.iter().find_map(|w| w.fixes.first()) {
        rounds += 1;
        if rounds > 10 {
            return Err(format!("fixes never stop coming:\n{}", shown(&files)));
        }
        let fixed = fixed(&files, &run, fix);
        let after = self::run(&fixed);
        if !same_behaviour(&after, &run) {
            return Err(format!(
                "fix {:?} changed what the program does:\n{}",
                fix.title,
                shown(&fixed)
            ));
        }
        (files, run) = (fixed, after);
    }
    if let Err(e) = &run.result
        && let Some(fix) = e.fixes.first()
    {
        let fixed = fixed(&files, &run, fix);
        if let Err(after) = self::run(&fixed).result
            && after.message == e.message
            && after.span == e.span
        {
            return Err(format!(
                "fix {:?} didn't fix the error:\n{}",
                fix.title,
                shown(&fixed)
            ));
        }
    }
    Ok(())
}

/// The files of the program that starts at `root`: it, and every `.t` file
/// in its directory and the directories in it, if it's `main.t`.
pub fn files_of(root: &Path) -> Files {
    let mut files = vec![(root.to_path_buf(), std::fs::read_to_string(root).unwrap())];
    if root
        .file_name()
        .is_some_and(|n| n == "main.t" || n == "lib.t")
    {
        let mut dirs = vec![root.parent().unwrap().to_path_buf()];
        while let Some(dir) = dirs.pop() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            entries.sort();
            for p in entries {
                if p.is_dir() {
                    dirs.push(p);
                } else if (p.extension().is_some_and(|e| e == "t")
                    || p.file_name().is_some_and(|n| n == "t.toml"))
                    && p != root
                {
                    let src = std::fs::read_to_string(&p).unwrap();
                    files.push((p, src));
                }
            }
        }
    }
    files
}

/// Do two runs print the same, and end the same way?
pub fn same_behaviour(a: &RunResult, b: &RunResult) -> bool {
    a.output == b.output
        && a.errors == b.errors
        && match (&a.result, &b.result) {
            (Ok(a), Ok(b)) => a == b,
            (Err(a), Err(b)) => a.message == b.message,
            _ => false,
        }
}

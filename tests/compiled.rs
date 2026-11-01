//! Differential tests of the C backend (docs/33-c-backend.md): every
//! program of `tests/t`, `examples/` and the docs (a ```` ```rust ````
//! block with a `fn main`, as `docs.rs` runs) that runs is also compiled, through
//! C, and run, with the same input, and must do exactly what the
//! interpreter does: write the same to its output and to where errors go,
//! a panic's message and where it's shown included, and end the same way.
//!
//! Left out are runs that end where only the interpreter can say: an
//! `unsafe` block breaking its obligations, which is undefined behaviour
//! in a compiled program, as `unsafe` means; and the interpreter's own
//! limits, on how much memory and how deep a stack it gives a program.
//!
//! It needs a C compiler, `$CC` or `cc`. `$T_LANG_CFLAGS` adds flags, as
//! for `t build`: CI runs this again with
//! `-fsanitize=address,undefined -fno-sanitize-recover=all`.

#[path = "shared/expect.rs"]
mod expect;

use expect::Expectations;
use std::fs;
use std::path::{Path, PathBuf};

/// A program to run both ways: what it's called, and its files.
struct Program {
    name: String,
    files: expect::Files,
}

/// Every program of `tests/t`, `examples/` and the docs.
fn programs() -> Vec<Program> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out: Vec<Program> = (files())
        .into_iter()
        .map(|p| Program {
            name: p.strip_prefix(root).unwrap_or(&p).display().to_string(),
            files: expect::files_of(&p),
        })
        .collect();
    let mut docs: Vec<PathBuf> = fs::read_dir(root.join("docs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    docs.sort();
    docs.insert(0, root.join("README.md"));
    for doc in docs {
        let src = fs::read_to_string(&doc).unwrap();
        let mut lines = src.lines().enumerate();
        while let Some((i, line)) = lines.next() {
            let indent = line.len() - line.trim_start().len();
            let Some(info) = line.trim_start().strip_prefix("```") else {
                continue;
            };
            let mut code = String::new();
            for (_, line) in lines.by_ref() {
                if line.trim_start().starts_with("```") {
                    break;
                }
                code.push_str(line.get(indent..).unwrap_or(""));
                code.push('\n');
            }
            // Only whole programs, which `docs.rs` runs.
            if info.trim() == "rust" && code.contains("fn main") {
                let name = doc.strip_prefix(root).unwrap_or(&doc).display();
                out.push(Program {
                    name: format!("{name}:{}", i + 1),
                    files: vec![(PathBuf::from("main.t"), code)],
                });
            }
        }
    }
    out
}

/// The first file of every program in `tests/t` and `examples/`.
fn files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    for dir in ["tests/t", "examples"] {
        for entry in fs::read_dir(root.join(dir)).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                if p.join("main.t").exists() {
                    paths.push(p.join("main.t"));
                }
            } else if p.extension().is_some_and(|e| e == "t") {
                paths.push(p);
            }
        }
    }
    paths.sort();
    paths
}

/// `f` of each of `paths`, on as many threads as there are cores.
fn on_all_cores<P: Sync, R: Send>(paths: &[P], f: impl Fn(&P) -> R + Sync) -> Vec<R> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut results: Vec<(usize, R)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads.min(paths.len()))
            .map(|_| {
                s.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(i) else {
                            return done;
                        };
                        done.push((i, f(path)));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    results.sort_by_key(|&(i, _)| i);
    results.into_iter().map(|(_, r)| r).collect()
}

/// The lines of `bytes`, as `RunResult` has them.
fn lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::to_string)
        .collect()
}

/// What happened to a program.
enum Outcome {
    /// It ran both ways, the same.
    Same,
    /// Only the interpreter can say what it does, or it doesn't run: why.
    Skipped(&'static str),
    Differs(String),
}

/// Runs program `p` both ways, compiled into `dir`.
fn differential(p: &Program, dir: &Path) -> Outcome {
    let files = &p.files;
    let all: String = files
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let input = Expectations::of(&all).input;
    let (root, src) = &files[0];
    let run = t_lang::run_files(root, src, Some(files.clone()), Some(input.clone()));
    let mut want_err = run.errors.clone();
    match &run.result {
        Err(e) if e.phase != t_lang::error::Phase::Runtime => {
            return Outcome::Skipped("doesn't check");
        }
        Err(e) => {
            let only = [
                ("broke its obligation", "breaks an obligation"),
                ("out of memory for", "runs out of the interpreter's memory"),
                ("stack overflow", "runs out of the interpreter's stack"),
            ];
            if let Some((_, why)) = only.iter().find(|(m, _)| e.message.contains(m)) {
                return Outcome::Skipped(why);
            }
            want_err.extend(e.render_in(&run.sources).lines().map(str::to_string));
        }
        Ok(_) => {}
    }
    let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
        Some((_, text)) => Ok(text.clone()),
        None => t_lang::no_files(p),
    };
    let options = t_lang::cgen::Options { lines: true };
    let c = match t_lang::build_c(root, src, &read, options).result {
        Ok(c) => c,
        Err(e) => return Outcome::Differs(format!("didn't build: {}", e.message)),
    };
    let name = p.name.replace(['/', '.', ':'], "_");
    let exe = dir.join(&name);
    if let Err(e) = t_lang::cgen::compile(&c, &exe) {
        let c_file = dir.join(format!("{name}.c"));
        let _ = fs::write(&c_file, &c);
        return Outcome::Differs(format!("{e}(the C is in {})", c_file.display()));
    }
    let mut child = std::process::Command::new(&exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        let _ = stdin.write_all(&input);
    }
    let out = child.wait_with_output().unwrap();
    let mut diffs = Vec::new();
    let (got_out, got_err) = (lines(&out.stdout), lines(&out.stderr));
    if got_out != run.output {
        diffs.push(format!(
            "output\n  interpreted: {:?}\n  compiled:    {got_out:?}",
            run.output
        ));
    }
    if got_err != want_err {
        diffs.push(format!(
            "errors\n  interpreted: {want_err:?}\n  compiled:    {got_err:?}"
        ));
    }
    let want_code = if run.failed() { 1 } else { 0 };
    if out.status.code() != Some(want_code) {
        diffs.push(format!(
            "ended with {} where the interpreter's {want_code}",
            out.status
        ));
    }
    let _ = fs::remove_file(&exe);
    match diffs.is_empty() {
        true => Outcome::Same,
        false => {
            let c_file = dir.join(format!("{name}.c"));
            let _ = fs::write(&c_file, &c);
            diffs.push(format!("(the C is in {})", c_file.display()));
            Outcome::Differs(diffs.join("\n"))
        }
    }
}

#[test]
#[cfg_attr(windows, ignore = "the C backend is for Linux and macOS, for now")]
fn compiled_runs_as_interpreted() {
    let dir = std::env::temp_dir().join(format!("t-lang-compiled-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let paths = programs();
    let outcomes = on_all_cores(&paths, |p| {
        std::panic::catch_unwind(|| differential(p, &dir)).unwrap_or_else(|e| {
            let why = (e.downcast_ref::<String>().cloned())
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            Outcome::Differs(format!("the backend panicked: {why}"))
        })
    });
    let mut failures = Vec::new();
    let (mut same, mut skipped) = (0, std::collections::BTreeMap::new());
    for (p, o) in paths.iter().zip(outcomes) {
        match o {
            Outcome::Same => same += 1,
            Outcome::Skipped(why) => *skipped.entry(why).or_insert(0) += 1,
            Outcome::Differs(why) => failures.push(format!("{}: {why}", p.name)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} programs ran differently compiled ({same} the same, skipped: {skipped:?}):\n\n{}\n",
        failures.len(),
        paths.len(),
        failures.join("\n\n")
    );
    let _ = fs::remove_dir_all(&dir);
    eprintln!("{same} programs ran the same both ways; skipped: {skipped:?}");
    assert!(same > 500, "only {same} programs ran both ways");
}

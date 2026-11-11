//! The refinement checker on MIR against the one on the AST
//! (docs/65-refinements-on-mir.md): each program in `tests/t` and
//! `examples`, and the libraries, is checked with both walks over each
//! body (`Refiner::Compare`), and they must find the same: the same proofs
//! failed, with the same messages, at the same places, and the same
//! operators proved not to panic. Where the walk over MIR proves more,
//! for a reason the AST's can't see, or explains a failed proof in other
//! words, it must be listed in `tests/refine_mir.allow`, a line each: the
//! program's name, the function's symbol, and why, after a `#`. Nothing
//! else may differ.
//!
//! Run with `PRINT_DIFFERENCES=1` to print every difference found.

#[path = "shared/expect.rs"]
mod expect;

use std::fs;
use std::path::{Path, PathBuf};
use t_lang::check::{Differs, Refiner, differences, set_refiner};

/// Every program: `tests/t/*.t`, `tests/t/*/main.t` or `lib.t`, and
/// `examples/*.t`.
fn programs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    for dir in ["tests/t", "examples"] {
        for entry in fs::read_dir(root.join(dir)).unwrap() {
            let p = entry.unwrap().path();
            match p.is_dir() {
                true if p.join("main.t").exists() => paths.push(p.join("main.t")),
                true if p.join("lib.t").exists() => paths.push(p.join("lib.t")),
                true => {}
                false if p.extension().is_some_and(|e| e == "t") => paths.push(p),
                false => {}
            }
        }
    }
    paths.sort();
    paths
}

/// What a program is called: its file's name, or its directory's.
fn name(path: &Path) -> String {
    let path = match path
        .file_name()
        .is_some_and(|n| n == "main.t" || n == "lib.t")
    {
        true => path.parent().unwrap(),
        false => path,
    };
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// The differences found checking the program at `path`.
fn check(path: &Path) -> Vec<t_lang::check::Difference> {
    let files = expect::files_of(path);
    let (first, src) = &files[0];
    let read = |p: &Path| match files.iter().find(|(f, _)| f == p) {
        Some((_, text)) => Ok(text.clone()),
        None => t_lang::no_files(p),
    };
    let _ = t_lang::check_files(first, src, &read);
    differences()
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn walks_agree() {
    set_refiner(Refiner::Compare);
    let paths = programs();
    let found: Vec<(String, Vec<t_lang::check::Difference>)> = std::thread::scope(|s| {
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let chunks: Vec<&[PathBuf]> = paths.chunks(paths.len().div_ceil(threads)).collect();
        let handles: Vec<_> = (chunks.into_iter())
            .map(|chunk| {
                s.spawn(move || {
                    (chunk.iter())
                        .map(|p| (name(p), check(p)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let allow =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/refine_mir.allow"))
            .unwrap_or_default();
    let allowed: Vec<(&str, &str)> = (allow.lines())
        .filter_map(|l| {
            let l = l.split('#').next()?.trim();
            let (program, function) = l.split_once(' ')?;
            Some((program, function.trim()))
        })
        .collect();
    let print = std::env::var_os("PRINT_DIFFERENCES").is_some();
    let mut failures = Vec::new();
    let mut used = Vec::new();
    for (program, diffs) in &found {
        for d in diffs {
            let at = (program.as_str(), d.function.as_str());
            let listed = allowed.contains(&at);
            let line = format!("{program}: {} ({:?}): {}", d.function, d.kind, d.what);
            if print {
                eprintln!("{line}");
            }
            match d.kind {
                Differs::Better | Differs::Wording if listed => used.push(at),
                _ => failures.push(line),
            }
        }
    }
    // Each line must still be needed.
    for at in &allowed {
        if !used.contains(at) {
            failures.push(format!("{} {}: listed, but no longer differs", at.0, at.1));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} programs differ:\n\n{}\n",
        found.iter().filter(|(_, d)| !d.is_empty()).count(),
        found.len(),
        failures.join("\n")
    );
}

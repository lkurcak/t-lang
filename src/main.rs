use std::process::ExitCode;
use t_lang::error::Warning;

const USAGE: &str = "usage: t [run|check|mono] <file.t>\n       t test <file.t> [filter]\n       t fmt [--check] <file.t>...\n       t lsp\n       t tree-sitter [--helix] <dir>";

const HELP: &str = "\
T: like C, but a different letter.

usage: t [run|check|mono] <file.t>
       t test <file.t> [filter]
       t fmt [--check] <file.t>...
       t lsp
       t tree-sitter [--helix] <dir>

  t <file.t>          type-check and run
  t check <file.t>    type-check only
  t test <file.t>     type-check, and run each `#[test]` of the program or
                      library, each as a program of its own (with a filter:
                      only those whose name has it in it)
  t mono <file.t>     print the program after monomorphisation
  --deny-warnings     with `check` or `test`: fail if there are warnings
  t fmt <file.t>...   format the files in place (--check: just print a diff)
  t lsp               language server, over stdio
  t tree-sitter <dir> install T's tree-sitter parser and highlight queries
                      into <dir>/parser and <dir>/queries/t, as Neovim
                      looks for them in its `site` directory (--helix:
                      the parser into <dir>/grammars, as Helix looks for
                      it in its `runtime` directory)
  t --help, -h        print this
  t --version, -V     print the version";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [flag] = args.as_slice() {
        match flag.as_str() {
            "--help" | "-h" => {
                println!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("t {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            _ => {}
        }
    }
    if let [cmd] = args.as_slice()
        && cmd == "lsp"
    {
        return match t_lang::lsp::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("t lsp: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if let [cmd, rest @ ..] = args.as_slice()
        && cmd == "tree-sitter"
    {
        return match rest {
            [dir] if !dir.starts_with('-') => {
                tree_sitter(std::path::Path::new(dir), Editor::Neovim)
            }
            [flag, dir] if flag == "--helix" && !dir.starts_with('-') => {
                tree_sitter(std::path::Path::new(dir), Editor::Helix)
            }
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    if let [cmd, rest @ ..] = args.as_slice()
        && cmd == "fmt"
    {
        return fmt(rest);
    }
    // `--deny-warnings`, wherever it is after `check` or `test`.
    let deny = matches!(args.first().map(String::as_str), Some("check" | "test"))
        && args.iter().any(|a| a == "--deny-warnings");
    let args: Vec<String> = args.into_iter().filter(|a| !deny || a != "--deny-warnings").collect();
    if let [cmd, rest @ ..] = args.as_slice()
        && cmd == "test"
    {
        return match rest {
            [path] if !path.starts_with('-') => test(path, None, deny),
            [path, filter] if !path.starts_with('-') && !filter.starts_with('-') => {
                test(path, Some(filter), deny)
            }
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    let (cmd, path) = match args.as_slice() {
        [cmd, path] if matches!(cmd.as_str(), "run" | "check" | "mono") => (cmd.as_str(), path),
        [path] if !path.starts_with('-') => ("run", path),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("error: cannot read `{path}`: {e}");
            return ExitCode::from(2);
        }
    };

    let path = std::path::Path::new(path);
    if cmd == "check" || cmd == "mono" {
        let checked = t_lang::check_files(path, &src, &t_lang::load::read_file);
        for w in Warning::render_all_in(&checked.warnings, &checked.sources) {
            eprintln!("{w}\n");
        }
        return match checked.result {
            Ok(_) if deny && !checked.warnings.is_empty() => {
                eprintln!("{}", denied(checked.warnings.len()));
                ExitCode::FAILURE
            }
            Ok(program) => {
                // The program with every generic function and type replaced
                // by its copies, as the interpreter runs it.
                if cmd == "mono" {
                    print!("{}", program.lower());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", e.render_in(&checked.sources));
                ExitCode::FAILURE
            }
        };
    }

    let run = t_lang::run_process(path, &src, |sources, warnings| {
        for w in Warning::render_all_in(warnings, sources) {
            eprintln!("{w}\n");
        }
    });
    match &run.result {
        // `main` gave back an error, and reported it.
        Ok(_) if run.failed() => ExitCode::FAILURE,
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e.render_in(&run.sources));
            ExitCode::FAILURE
        }
    }
}

/// What `--deny-warnings` says when there were `n` warnings.
fn denied(n: usize) -> String {
    match n {
        1 => "error: there was a warning, and `--deny-warnings` says that fails".to_string(),
        n => format!("error: there were {n} warnings, and `--deny-warnings` says that fails"),
    }
}

/// `t test <file> [filter]` checks the program or library at `path`, then
/// runs its tests, those whose name has `filter` in it if it's given, and
/// says how each went: a line each as it goes, then why each that failed
/// did, then how many passed and failed. It fails if any failed, if the
/// program doesn't check, or, if `deny`, if there were warnings.
fn test(path: &str, filter: Option<&str>, deny: bool) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("error: cannot read `{path}`: {e}");
            return ExitCode::from(2);
        }
    };
    let run = t_lang::test_files(
        std::path::Path::new(path),
        &src,
        &t_lang::load::read_file,
        filter,
    );
    for w in Warning::render_all_in(&run.warnings, &run.sources) {
        eprintln!("{w}\n");
    }
    let tests = match &run.result {
        Ok(tests) => tests,
        Err(e) => {
            eprintln!("{}", e.render_in(&run.sources));
            return ExitCode::FAILURE;
        }
    };
    for t in tests {
        let how = if t.failure.is_some() { "FAILED" } else { "ok" };
        println!("test {} ... {how}", t.name);
    }
    let failed: Vec<_> = tests.iter().filter(|t| t.failure.is_some()).collect();
    for t in &failed {
        let e = t.failure.as_ref().expect("a failure");
        println!("\n--- {}\n{}", t.name, e.render_in(&run.sources));
    }
    let passed = tests.len() - failed.len();
    println!("\n{passed} passed; {} failed", failed.len());
    if deny && !run.warnings.is_empty() {
        eprintln!("{}", denied(run.warnings.len()));
        return ExitCode::FAILURE;
    }
    match failed.is_empty() {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    }
}

/// `t fmt <file>...` rewrites each file formatted. With `--check` it
/// rewrites nothing, and prints a diff for each file that isn't formatted.
fn fmt(args: &[String]) -> ExitCode {
    let check = args.iter().any(|a| a == "--check");
    let paths: Vec<&String> = args.iter().filter(|a| *a != "--check").collect();
    if paths.is_empty() || paths.iter().any(|p| p.starts_with('-')) {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut ok = true;
    for path in paths {
        let src = match std::fs::read_to_string(path) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("error: cannot read `{path}`: {e}");
                return ExitCode::from(2);
            }
        };
        let formatted = match t_lang::fmt::format(&src) {
            Ok(formatted) => formatted,
            Err(e) => {
                eprintln!("{}", e.render(path, &src));
                ok = false;
                continue;
            }
        };
        if formatted == src {
            continue;
        }
        if check {
            print!("{}", t_lang::fmt::diff(path, &src, &formatted));
            ok = false;
        } else if let Err(e) = std::fs::write(path, &formatted) {
            eprintln!("error: cannot write `{path}`: {e}");
            return ExitCode::from(2);
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The tree-sitter parser for `editors/tree-sitter-t`, built by `build.rs`,
/// and its highlight queries: those of this `t`, so what's highlighted is
/// what it accepts.
const TREE_SITTER_PARSER: &[u8] = include_bytes!(env!("T_TREE_SITTER_PARSER"));
const TREE_SITTER_HIGHLIGHTS: &str =
    include_str!("../editors/tree-sitter-t/queries/highlights.scm");

/// Where `t tree-sitter` puts the parser: editors agree on where the
/// queries go, but not on the parser's directory or file name.
enum Editor {
    Neovim,
    Helix,
}

/// `t tree-sitter <dir>` writes the parser to `<dir>/parser/t.so` and the
/// highlight queries to `<dir>/queries/t/highlights.scm`; with `--helix`,
/// the parser to `<dir>/grammars/t.so` (`t.dll` on Windows). Files that are
/// already as they'd be written are left alone, so running it on every
/// start is cheap and an editor can tell whether anything changed.
fn tree_sitter(dir: &std::path::Path, editor: Editor) -> ExitCode {
    let parser = match editor {
        // Neovim loads `parser/t.*`, whatever the extension.
        Editor::Neovim => "parser/t.so",
        Editor::Helix if cfg!(windows) => "grammars/t.dll",
        Editor::Helix => "grammars/t.so",
    };
    let files: [(&str, &[u8]); 2] = [
        (parser, TREE_SITTER_PARSER),
        (
            "queries/t/highlights.scm",
            TREE_SITTER_HIGHLIGHTS.as_bytes(),
        ),
    ];
    for (name, contents) in files {
        let path = dir.join(name);
        if std::fs::read(&path).is_ok_and(|old| old == contents) {
            continue;
        }
        if let Err(e) = write_replacing(&path, contents) {
            eprintln!("error: cannot write `{}`: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Writes `path` by renaming a new file over it, rather than rewriting it in
/// place: an editor may have the old parser loaded, and changing the file
/// under it can crash it.
fn write_replacing(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

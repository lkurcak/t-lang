use std::process::ExitCode;
use t_lang::error::Warning;
use t_lang::interp::Value;

const USAGE: &str =
    "usage: t [run|check|mono] <file.t>\n       t fmt [--check] <file.t>...\n       t lsp";

const HELP: &str = "\
T: like C, but a different letter.

usage: t [run|check|mono] <file.t>
       t fmt [--check] <file.t>...
       t lsp

  t <file.t>          type-check and run
  t check <file.t>    type-check only
  t mono <file.t>     print the program after monomorphisation
  t fmt <file.t>...   format the files in place (--check: just print a diff)
  t lsp               language server, over stdio
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
        && cmd == "fmt"
    {
        return fmt(rest);
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
            Ok(program) => {
                // The program with every generic function and type replaced
                // by its copies, as the interpreter runs it.
                if cmd == "mono" {
                    print!("{program}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", e.render_in(&checked.sources));
                ExitCode::FAILURE
            }
        };
    }

    let run = t_lang::run_files(path, &src, None);
    for w in Warning::render_all_in(&run.warnings, &run.sources) {
        eprintln!("{w}\n");
    }
    for line in &run.output {
        println!("{line}");
    }
    match run.result {
        Ok(Value::Unit) => ExitCode::SUCCESS,
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e.render_in(&run.sources));
            ExitCode::FAILURE
        }
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

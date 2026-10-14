use std::process::ExitCode;
use t_lang::interp::Value;

const USAGE: &str = "usage: t [run|check|mono] <file.t>\n       t lsp";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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

    if cmd == "check" || cmd == "mono" {
        let checked = t_lang::check_source(&src);
        for w in &checked.warnings {
            eprintln!("{}\n", w.render(path, &src));
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
                eprintln!("{}", e.render(path, &src));
                ExitCode::FAILURE
            }
        };
    }

    let run = t_lang::run_source(&src);
    for w in &run.warnings {
        eprintln!("{}\n", w.render(path, &src));
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
            eprintln!("{}", e.render(path, &src));
            ExitCode::FAILURE
        }
    }
}

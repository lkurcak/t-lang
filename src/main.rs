use std::process::ExitCode;
use t::interp::Value;

const USAGE: &str = "usage: t [run|check] <file.t>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, path) = match args.as_slice() {
        [cmd, path] if cmd == "run" || cmd == "check" => (cmd.as_str(), path),
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

    if cmd == "check" {
        return match t::check_source(&src) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", e.render(path, &src));
                ExitCode::FAILURE
            }
        };
    }

    let run = t::run_source(&src);
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

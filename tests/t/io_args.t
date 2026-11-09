// The arguments a program is started with: its name apart, then each
// word, as a `&'static str`. `Args` is `Copy`: reading them changes
// nothing, so anyone may.
use std::env::Args;

fn second(args: Args) -> &'static str { args.get(1).unwrap_or("none") }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let args = os.args;
    out.line(&args.program())?; // expect: t-program
    out.line(&args.len())?;     // expect: 3
    for a in args.iter() {
        out.put("[")?;
        out.put(&a)?;
        out.line("]")?;
    }
    // expect: [hello]
    // expect: [wörld]
    // expect: [--flag]
    out.line(&second(args))?;          // expect: wörld
    out.line(&args.get(3).is_none())?; // expect: true
    if args.len() > 2 {
        out.line(&args.index(2))?; // expect: --flag
    }
    out.line(&os.args.is_empty()) // expect: false
}
// arg: hello
// arg: wörld
// arg: --flag

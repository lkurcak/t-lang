// Functions are values: they can be passed, returned and stored in variables.

fn inc(x: i64) -> i64 with panic { x + 1 }
fn double(x: i64) -> i64 with panic { x * 2 }

fn apply(f: fn(i64) -> i64 with panic, x: i64) -> i64 with panic { f(x) }
fn twice(f: fn(i64) -> i64 with panic, x: i64) -> i64 with panic { f(f(x)) }

fn pick(first: bool) -> fn(i64) -> i64 with panic {
    if first { inc } else { double }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&apply(inc, 1))?;           // expect: 2
    out.line(&twice(double, 3))?;        // expect: 12
    out.line(&pick(false)(10))?;         // expect: 20
    let p: fn(i64) -> () = print;   // even builtins are values
    p(7);                           // expect: 7
    Result::Ok(())
}

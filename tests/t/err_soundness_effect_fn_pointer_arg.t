// Function types only gain effects: passing one that may panic where one
// that can't is wanted is no more allowed than storing it.
fn call(f: fn() -> i64) -> i64 { f() }

fn boom() -> i64 with panic { panic("stop") }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&call(boom))?;
    Result::Ok(())
}
// error: expected `fn() -> i64`, found `fn() -> i64 with panic`

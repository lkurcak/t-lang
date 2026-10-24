// A closure that captures isn't a plain function: it carries what it
// captures. Take it as a type parameter that implements `Fn`.
fn apply(f: fn(i64) -> i64, x: i64) -> i64 { f(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let k = 10;
    out.line(&apply(|x| x * k, 4))?;
    Result::Ok(())
}
// error: expected `fn(i64) -> i64`, found a closure that captures `k`

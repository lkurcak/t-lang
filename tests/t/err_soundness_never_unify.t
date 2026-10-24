// A `fn() -> !` isn't a `fn() -> i64`, or the other way round: a call of
// one is taken never to return, which would let a value that isn't used up
// go.
fn die() -> ! {
    loop {}
}

fn one() -> i64 { 1 }

fn pick<T>(a: fn() -> T, _b: fn() -> T) -> T { a() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&pick(one, die))?;
    Result::Ok(())
}
// error: expected `fn() -> i64`, found `fn() -> !`

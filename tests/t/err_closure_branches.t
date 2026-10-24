// Each closure that captures has a type of its own, which isn't a function
// type, so branches can't give one and a function.
fn pick(n: i64, b: bool) -> i64 {
    let f = if b { |x: i64| x + n } else { |x: i64| x - 1 };
    f(1)
}
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&pick(1, false))?;  Result::Ok(()) }
// error: `if` and `else` have different types: `{closure}` and `fn(i64) -> i64`

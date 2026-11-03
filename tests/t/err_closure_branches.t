// Each closure has a type of its own, which isn't a function type, so
// branches can't give two, even if one captures nothing.
fn pick(n: i64, b: bool) -> i64 {
    let f = if b { |x: i64| x + n } else { |x: i64| x - 1 };
    f(1)
}
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&pick(1, false))?;  Result::Ok(()) }
// error: `if` and `else` have different types: `{closure}` and `{closure}`; each closure has a type of its own

// The same for the branches of an `if`: with no type wanted, a closure in
// one and a function in the other differ. (There's no "least upper bound":
// what's wanted decides, not which branch comes first.)
fn inc(x: i64) -> i64 { x + 1 }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = true;
    let f = if b { |x: i64| x * 2 } else { inc };
    out.line(&f(1))?;
    Result::Ok(())
}
// error: `if` and `else` have different types: `{closure}` and `fn(i64) -> i64

// A function that only takes `'static` references can't be used where one
// that takes any is wanted.
fn apply(f: fn(&i64) -> i64, x: &i64) -> i64 { f(x) }
fn only_static(r: &'static i64) -> i64 { *r }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 1;
    out.line(&apply(only_static, &n))?;
    Result::Ok(())
}
// error: this function isn't general enough

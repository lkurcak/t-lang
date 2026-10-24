// A variable declared `mut` that's never changed. The fix removes the `mut`.

fn count(mut n: i64) -> i64 with panic { n + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut x = 1;
    let mut y = 2;
    y += x;
    out.line(&y)?;           // expect: 3
    out.line(&count(x))?;    // expect: 2
    let f = |mut z: i64| z * 2;
    out.line(&f(2))?;        // expect: 4
    Result::Ok(())
}
// warning: variable `n` doesn't need to be mutable
// warning: variable `x` doesn't need to be mutable
// warning: variable `z` doesn't need to be mutable

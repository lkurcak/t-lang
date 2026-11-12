// A closure made in a loop captures `x` by `&mut` and writes 0: the
// capture is a loan the loop makes each time round, so the head doesn't
// know `x` is still 1.
fn f() -> i64 with {} {
    let mut x: i64 = 1;
    let mut n: i64 = 0;
    while n < 3 {
        let _y = 10 / x;
        let mut z = || x = 0;
        z();
        n = n + 1;
    }
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f())?;
    Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`

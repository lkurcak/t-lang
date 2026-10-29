// Through `&mut d`, `zero` changes `d`.
fn zero(r: &mut i64) { *r = 0; }

fn f() -> i64 with {} {
    let mut d = 1;
    zero(&mut d);
    10 / d
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f())?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`

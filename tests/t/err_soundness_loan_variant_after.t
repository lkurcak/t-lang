// A `&mut` binding into a variant's field, written through: after the
// `match`, what's in `*o` is what the write left, not what the test before
// it saw.
fn f(o: &mut Option<i64>) -> i64 with {} {
    let Some(v) = *o else { return 0 };
    if v > 0 {
        match o {
            Some(n) => *n = 0,
            None => {}
        }
        match *o {
            Some(v) => 10 / v,
            None => 0,
        }
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut o = Some(3);
    out.line(&f(&mut o))?;
    Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`

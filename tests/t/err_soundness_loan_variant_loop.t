// A pattern binds a variant's field by `&mut` in a loop, and writes 0
// through it: the next time round, what's in the variant isn't what it was
// before the loop. The walk can't name a variant's field as a place, so the
// binding forgets the nearest place it can name, `*o`, which the loop then
// changes.
fn f(o: &mut Option<i64>) -> i64 with {} {
    *o = Some(1);
    let mut k: i64 = 0;
    while k < 2 {
        match *o {
            Some(v) => {
                let _w = 10 / v;
            }
            None => {}
        }
        match o {
            Some(n) => {
                *n = 0;
            }
            None => {}
        }
        k = k + 1;
    }
    0
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut o = None;
    out.line(&f(&mut o))?;
    Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`

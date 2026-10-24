// Through a generic function, `T` = `&'x i64` for one `'x`, so both
// arguments must live as long as the result.

use alloc::boxed::Box;

trait Pick {
    fn pick(self, other: Self) -> Self;
}

impl<'a> Pick for &'a i64 {
    fn pick(self, other: &'a i64) -> &'a i64 { other }
}

fn choose<T: Pick>(a: T, b: T) -> T { a.pick(b) }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let long = Box::new(1i64);
    let r;
    {
        let short = Box::new(2);
        r = choose(&*long, &*short);
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: `short` doesn't live long enough

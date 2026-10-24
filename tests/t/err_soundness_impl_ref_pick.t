// Through a generic function, `T` = `&'x i64` for one `'x`, so both
// arguments must live as long as the result.
trait Pick {
    fn pick(self, other: Self) -> Self;
}

impl<'a> Pick for &'a i64 {
    fn pick(self, other: &'a i64) -> &'a i64 { other }
}

fn choose<T: Pick>(a: T, b: T) -> T { a.pick(b) }

fn main() with alloc {
    let long = Box::new(1i64);
    let r;
    {
        let short = Box::new(2);
        r = choose(&*long, &*short);
    }
    print(*r);
}
// error: `short` doesn't live long enough

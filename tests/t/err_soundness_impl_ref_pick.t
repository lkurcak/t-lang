// Through a generic function, `T` = `&'x int` for one `'x`, so both
// arguments must live as long as the result.
trait Pick {
    fn pick(self, other: Self) -> Self;
}

impl<'a> Pick for &'a int {
    fn pick(self, other: &'a int) -> &'a int { other }
}

fn choose<T: Pick>(a: T, b: T) -> T { a.pick(b) }

fn main() {
    let long = Box::new(1);
    let r;
    {
        let short = Box::new(2);
        r = choose(&*long, &*short);
    }
    print(*r);
}
// error: `short` doesn't live long enough

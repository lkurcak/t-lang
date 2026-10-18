// What a projection stands for is worked out with the lifetimes of the
// type it's on, each in its place: `<Pair<'x, 'y> as Second>::Out` is
// `&'y int`, so it can't outlive what `'y` borrows. Finding the impl
// ignores lifetimes, but working out `Out` mustn't.
#[derive(Discard)]
struct Pair<'a, 'b> { a: &'a int, b: &'b int }

trait Second {
    type Out;

    fn second(self) -> Self::Out;
}

impl<'a, 'b> Second for Pair<'a, 'b> {
    type Out = &'b int;

    fn second(self) -> &'b int { self.b }
}

fn get<T: Second>(t: T) -> T::Out { t.second() }

fn main() {
    let long = 1;
    let r;
    {
        let short = 2;
        r = get(Pair { a: &long, b: &short });
    }
    print(*r);
}
// error: `short` doesn't live long enough

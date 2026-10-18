// The same through a method call, whose `Self::Out` is worked out at the
// lifetimes of the receiver's type.
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

fn main() {
    let long = 1;
    let r;
    {
        let short = 2;
        let p = Pair { a: &long, b: &short };
        r = p.second();
    }
    print(*r);
}
// error: `short` doesn't live long enough

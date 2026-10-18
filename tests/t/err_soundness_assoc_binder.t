// Finding an impl doesn't look at what a function type is generic over,
// so this impl is found for `fn(&int) -> &int`, with `A` and `B` its
// `&int`s, whose lifetime is a call's. As `In` and `Out`, that lifetime
// would mean nothing, and `call` would give back a reference that outlives
// what it points to. So an associated type can't use a parameter that's
// only inside a function type.
trait Tr {
    type In;
    type Out;

    fn call(self, x: Self::In) -> Self::Out;
}

impl<A, B> Tr for fn(A) -> B {
    type In = A;
    type Out = B;

    fn call(self, x: A) -> B { self(x) }
}

fn id(x: &int) -> &int { x }

fn main() {
    let r;
    {
        let n = 5;
        let f: fn(&int) -> &int = id;
        r = Tr::call(f, &n);
    }
    print(*r);
}
// error: `A` is only inside a function type in `fn(A) -> B`

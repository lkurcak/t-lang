// One impl, used at two lifetimes in one body: each use's `Out` has its
// own. Working it out once per type, with lifetimes erased, and reusing
// that, would give the second the first's.
trait Tr {
    type Out;

    fn out(self) -> Self::Out;
}

impl<'a> Tr for &'a int {
    type Out = &'a int;

    fn out(self) -> &'a int { self }
}

fn main() {
    let long = 1;
    let r1 = Tr::out(&long);
    let r2;
    {
        let short = 2;
        r2 = Tr::out(&short);
    }
    print(*r1 + *r2);
}
// error: `short` doesn't live long enough

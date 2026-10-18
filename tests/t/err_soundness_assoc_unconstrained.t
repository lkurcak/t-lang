// An impl lifetime that isn't in the impl's type would make `Out` a
// reference of any lifetime, a new one at each mention: `put` would then
// get a slot and a value whose types say nothing about each other, and
// write a short-lived reference into a long-lived slot.
#[derive(Discard)]
struct S { n: int }

trait Tr {
    type Out: Discard;
}

impl<'a> Tr for S {
    type Out = &'a int;
}

fn put<T: Tr>(slot: &mut T::Out, v: T::Out) {
    let _old = replace(slot, v);
}

fn main() {
    let long = 1;
    let mut slot = &long;
    {
        let short = 2;
        put::<S>(&mut slot, &short);
    }
    print(*slot);
}
// error: `'a` isn't in `S`

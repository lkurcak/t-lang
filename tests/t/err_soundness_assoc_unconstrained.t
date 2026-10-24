// An impl lifetime that isn't in the impl's type would make `Out` a
// reference of any lifetime, a new one at each mention: `put` would then
// get a slot and a value whose types say nothing about each other, and
// write a short-lived reference into a long-lived slot.

use core::mem::replace;

#[derive(Discard)]
struct S { n: i64 }

trait Tr {
    type Out: Discard;
}

impl<'a> Tr for S {
    type Out = &'a i64;
}

fn put<T: Tr>(slot: &mut T::Out, v: T::Out) {
    let _old = replace(slot, v);
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let long = 1;
    let mut slot = &long;
    {
        let short = 2;
        put::<S>(&mut slot, &short);
    }
    out.line(&(*slot))?;
    Result::Ok(())
}
// error: `'a` isn't in `S`

// What a closure's parameter is, is worked out after its body: here `*id(x)`
// is a `T` not known yet, which the call then makes `str`. A value of it
// would have no size.
fn id<'a, unsized T>(x: &'a T) -> &'a T { x }

fn main() {
    let c = |x| *id(x);
    let _s = c("abc");
}
// error: `str` can only be behind a shared reference, as in `&str`

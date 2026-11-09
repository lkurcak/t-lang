// A tuple is covariant in its values, but what's behind a `&mut` isn't:
// this would let `t.0` dangle.
fn put<'a>(t: &mut (&'static str, u8), s: &'a str) {
    t.0 = s;
}

fn main() {}
// error: lifetime may not live long enough: this needs `'a` to outlive `'static`

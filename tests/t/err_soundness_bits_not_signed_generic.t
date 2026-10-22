// Of a `T: Signed`, `!0` is -1 whichever it is, so `+ 1` fits; of a
// `T: Unsigned` it's the greatest, so `+ 1` doesn't.
fn f<T: Unsigned>() -> T { !0 + 1 }

fn main() {}
// error: `+` may panic, but `f` doesn't say `with panic`

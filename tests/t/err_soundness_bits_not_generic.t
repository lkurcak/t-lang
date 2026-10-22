// `!0` is -1 of a signed type, but the greatest value of an unsigned one,
// to which 1 can't be added.
fn f<T: Int>() -> T { !0 + 1 }

fn main() {}
// error: `+` may panic, but `f` doesn't say `with panic`

// An operator's left side is worked out first: 250, to which the 10 the
// right gives is added, whatever the right makes of its variable.
fn main() with {} {
    let mut a = 250u8;
    let _b = a + { a = 0; 10 };
}
// error: `+` may panic, but `main` doesn't say `with panic`

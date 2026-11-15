// A value lent `&mut` to a call is whatever the call left: `x == 1` was
// true of what it was before, not of what it is after.
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

fn bump(r: &mut u32) {
    *r = 9;
}

fn main() {
    let mut x: u32 = 1;
    bump(&mut x);
    if x > 5 {
        absurd(())
    }
}
// error: can't prove `false` for argument `_u` of `absurd`

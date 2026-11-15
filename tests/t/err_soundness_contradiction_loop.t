// What was known before a loop isn't known at its head, where `i` is any
// value the loop may have given it: `i == 0` and `i > 5` aren't both
// known there, so the branch isn't one where nothing runs.
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

fn main() {
    let mut i: u32 = 0;
    while i < 10 {
        if i > 5 {
            absurd(())
        }
        i += 1;
    }
}
// error: can't prove `false` for argument `_u` of `absurd`

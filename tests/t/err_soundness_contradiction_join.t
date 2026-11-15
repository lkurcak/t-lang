// Where two paths meet, what's known is what's true on both: `x == 1` on
// one and `x == 7` on the other leave `x == 1 || x == 7`, which `x > 5`
// doesn't contradict.
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

fn pick(b: bool) {
    let x: u32 = if b { 1 } else { 7 };
    if x > 5 { absurd(()) }
}

fn main() {
    pick(false);
}
// error: can't prove `false` for argument `_u` of `absurd`

// An arm is ruled out only by what's known of the value matched: here
// nothing is, so the arm may run.
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

#[derive(Discard)]
enum Small { Zero, One(u32) }

fn get(s: Small) -> u32 {
    match s {
        Small::Zero => 0,
        Small::One(_) => absurd(()),
    }
}

fn main() {
    let _n = get(Small::One(1));
}
// error: can't prove `false` for argument `_u` of `absurd`

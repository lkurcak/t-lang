// A closure's parameter is whatever it's called with: what's known where
// it's made says nothing of it.
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

fn main() {
    let x: u32 = 3;
    let c = |y: u32| {
        if y > 5 {
            absurd(())
        }
    };
    c(x);
    c(9);
}
// error: can't prove `false` for argument `_u` of `absurd`

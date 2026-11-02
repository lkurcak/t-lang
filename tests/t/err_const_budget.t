// Nothing says a loop ends, so working out a value has a budget of steps.
fn spin() -> u8 {
    loop {}
}

const N: u8 = spin();
// error: evaluating `N` took more than

fn main() {
    let _ = N;
}

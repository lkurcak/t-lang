// An effect is said once, with one condition: `||` joins two.
fn f(x: u8) -> u8 with {panic if x > 3, panic if x == 0} { x }

fn main() {}
// error: `panic` is said twice: say it once, with `||` between its conditions

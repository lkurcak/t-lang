// Called through a function value, nothing would check the argument.
fn div(a: i64, d: { d: i64 | d > 0 }) -> i64 { a / d }

fn apply(f: fn(i64, i64) -> i64, d: i64) -> i64 { f(1, d) }

fn main() {
    print(apply(div, 0));
}
// error: `div` can't be used as a value: its parameter `d` has a refinement, which a call through a value wouldn't check

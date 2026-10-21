// Called through a function value, nothing would check the argument.
fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }

fn apply(f: fn(int, int) -> int, d: int) -> int { f(1, d) }

fn main() {
    print(apply(div, 0));
}
// error: `div` can't be used as a value: its parameter `d` has a refinement, which a call through a value wouldn't check

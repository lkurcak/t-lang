// A result's refinement says what callers may count on, and can name the
// parameters.
fn clamp(x: int, hi: { h: int | h >= 0 }) -> { r: int | r <= hi } {
    if x < hi { x } else { hi }
}

fn at_least_one(n: int) -> { r: int | r >= 1 } {
    if n < 1 { 1 } else { n }
}

fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }

fn inc(x: int) -> { r: int | r == x + 1 } with panic { x + 1 }

fn main() with panic {
    print(clamp(12, 10)); // expect: 10
    print(div(9, at_least_one(-5))); // expect: 9
    print(div(9, at_least_one(3))); // expect: 3
    let y = inc(4);
    print(div(10, y - 4)); // expect: 10
}

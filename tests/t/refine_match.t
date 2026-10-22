// What a `match` on an `i64` matched is known in each arm, and so is what
// the arms before it didn't.
fn div_or_zero(a: i64, d: i64) -> i64 {
    match d {
        0 => 0,
        -1 => 0,
        _ => a / d,
    }
}

fn small_inc(n: i64) -> i64 {
    match n {
        0..=9 => n + 1,
        _ => 0,
    }
}

fn main() {
    print(div_or_zero(7, 0)); // expect: 0
    print(div_or_zero(7, 2)); // expect: 3
    print(small_inc(3)); // expect: 4
    print(small_inc(30)); // expect: 0
}

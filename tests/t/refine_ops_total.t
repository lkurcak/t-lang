// Operators the checker can prove don't overflow, or don't divide by
// zero, can't panic.
fn inc(x: { v: i64 | v < 100 }) -> i64 { x + 1 }

fn neg_five() -> i64 { -5 }

fn half(n: i64) -> i64 { n / 2 }

fn guarded(a: i64, d: i64) -> i64 {
    if d == 0 {
        return 0;
    }
    if d == -1 {
        return 0;
    }
    a / d
}

fn short_circuit(a: i64, d: i64) -> bool { d > 0 && a / d > 1 }

fn sum_small(a: { v: i64 | v >= 0 && v <= 1000 }, b: { v: i64 | v >= -1000 && v <= 0 }) -> i64 {
    a + b + a * 3 - b
}

fn main() {
    print(inc(41)); // expect: 42
    print(neg_five()); // expect: -5
    print(half(9)); // expect: 4
    print(guarded(10, 0)); // expect: 0
    print(guarded(10, 3)); // expect: 3
    print_bool(short_circuit(10, 0)); // expect: false
    print_bool(short_circuit(10, 3)); // expect: true
    print(sum_small(10, -5)); // expect: 40
    print(1i64 + 2 * 3); // expect: 7
}

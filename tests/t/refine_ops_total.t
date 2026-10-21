// Operators the checker can prove don't overflow, or don't divide by
// zero, can't panic.
fn inc(x: { v: int | v < 100 }) -> int { x + 1 }

fn neg_five() -> int { -5 }

fn half(n: int) -> int { n / 2 }

fn guarded(a: int, d: int) -> int {
    if d == 0 {
        return 0;
    }
    if d == -1 {
        return 0;
    }
    a / d
}

fn short_circuit(a: int, d: int) -> bool { d > 0 && a / d > 1 }

fn sum_small(a: { v: int | v >= 0 && v <= 1000 }, b: { v: int | v >= -1000 && v <= 0 }) -> int {
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
    print(1 + 2 * 3); // expect: 7
}

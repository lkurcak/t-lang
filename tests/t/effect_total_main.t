// A `main` with no `with` is a program that can't panic or allocate.
fn max(a: int, b: int) -> int { if a < b { b } else { a } }

fn main() {
    print(max(1, 2)); // expect: 2
    let o = checked_add(9223372036854775807, 1);
    print_bool(o.is_none()); // expect: true
    print_bool(checked_div(1, 0).is_none()); // expect: true
    print_bool(checked_neg(5).is_some()); // expect: true
    // A negative literal is a constant: `-` of a literal can't overflow.
    print(-9223372036854775807); // expect: -9223372036854775807
}

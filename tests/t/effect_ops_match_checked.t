// The operators are what `checked_*` and `panic()` make of them.
fn add(a: int, b: int) -> int with panic {
    match checked_add(a, b) {
        Option::Some(c) => c,
        Option::None => panic(),
    }
}

fn main() with panic {
    let big = 9223372036854775807;
    print_bool(add(1, 2) == 1 + 2); // expect: true
    print_bool(checked_sub(-big - 1, 1).is_none()); // expect: true
    print_bool(checked_mul(big, 2).is_none()); // expect: true
    print_bool(checked_div(-big - 1, -1).is_none()); // expect: true
    print_bool(checked_rem(7, 0).is_none()); // expect: true
    print_bool(checked_rem(-7, 2).unwrap() == -7 % 2); // expect: true
    print_bool(checked_neg(-big - 1).is_none()); // expect: true
}

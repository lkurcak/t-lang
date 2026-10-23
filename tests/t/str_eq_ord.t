// Strings compare by their bytes, as do slices of what compares. `<` is
// for integers; strings are ordered by `Ord`.
use core::cmp::Ordering;

fn main() {
    print_bool("abc" == "abc"); // expect: true
    print_bool("abc" != "abd"); // expect: true
    print_bool("ab".lt("abc")); // expect: true
    print_bool("b".gt("abc")); // expect: true
    print_bool(b"xy" == b"xy"); // expect: true
    let x: &[u8] = b"x";
    print_bool(x == b"y"); // expect: false
    match "é".cmp("z") {
        Ordering::Greater => print(1u8), // expect: 1
        _ => print(0u8),
    }
}

// A `match` on an integer covers its type's range: all of a `u8` is
// `0..=255`, and an open end runs to its type's end.
fn kind(b: u8) -> u8 {
    match b {
        0..=31 => 0,
        32..=126 => 1,
        127.. => 2,
    }
}

fn sign(n: i8) -> i8 {
    match n {
        -128..=-1 => -1,
        0 => 0,
        1..=127 => 1,
    }
}

fn main() {
    print(kind(10));    // expect: 0
    print(kind(65));    // expect: 1
    print(kind(255));   // expect: 2
    print(sign(-128));  // expect: -1
    print(sign(0));     // expect: 0
    print(sign(5));     // expect: 1
}

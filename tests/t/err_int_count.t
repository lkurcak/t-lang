// `count` gives a `usize`, which is whatever the iterator counted.
fn main() with panic {
    let r: core::ops::Range<i8> = -128..127;
    print(r.count());   // expect: 255
    let none: u8 = 5;
    print(none - 6);
}
// error: runtime error: integer overflow

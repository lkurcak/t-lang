// A `Range<u8>` stops before its end, so counting up to 255 never adds
// one to 255.
fn main() with panic {
    let end: u8 = 255;
    let mut n: u16 = 0;
    for i in 250..end {
        n = n + (i as u16);
    }
    print(n);   // expect: 1260
    let m: i64 = core::num::greatest::<u8>() as i64 + 1;
    print(m);   // expect: 256
    let x: u8 = 255;
    print(x + 1);
}
// error: runtime error: integer overflow

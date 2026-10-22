// `!` of an integer flips every bit: of an unsigned type that's its
// greatest value minus it, of a signed one -1 minus it. Of a `bool`, it's
// as before.
fn flip<T: Int>(x: T) -> T { !x }

fn main() {
    let z: u8 = 0;
    print(!z);              // expect: 255
    print(!0x0Fu16);        // expect: 65520
    let s: i8 = 0;
    print(!s);              // expect: -1
    print(!-128i8);         // expect: 127
    print(flip(5u32));      // expect: 4294967290
    print(flip(5i64));      // expect: -6
    print_bool(!false);     // expect: true
    let f = |x| !x;
    print(f(1u8));          // expect: 254
}

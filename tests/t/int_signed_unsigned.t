// `Signed` and `Unsigned` are `Int`'s two halves. Of a `T: Unsigned`, the
// least is 0 and a literal may be up to 255; of a `T: Signed`, the least
// is -1 minus the greatest and a literal may be from -128 to 127.
use core::num::{abs, checked_abs, wrapping_abs};

fn byte_max<T: Unsigned>() -> T { 255 }

fn minus_one<T: Signed>() -> T { -1 }

fn not_negative<T: Unsigned>(x: T) -> bool { x >= 0 }

// `!0` is -1 of every signed type, so this can't panic.
fn zero<T: Signed>() -> T { !0 + 1 }

fn pred<T: Unsigned>(x: T) -> T { if x > 0 { x - 1 } else { x } }

fn main() with panic {
    print(byte_max::<u16>());           // expect: 255
    print(minus_one::<i32>());          // expect: -1
    print_bool(not_negative(3u8));      // expect: true
    print(zero::<i16>());               // expect: 0
    print(pred(0u64));                  // expect: 0
    print(pred(7usize));                // expect: 6
    print(abs(-5i16));                  // expect: 5
    print(checked_abs(-128i8).unwrap_or(0)); // expect: 0
    print(checked_abs(-127i8).unwrap_or(0)); // expect: 127
    print(wrapping_abs(-128i8));        // expect: -128
    print(wrapping_abs(-3i64));         // expect: 3
}

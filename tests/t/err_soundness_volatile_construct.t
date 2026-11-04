// A volatile value is only ever at an address `at` was given: there's no
// making one.
use core::volatile::Volatile;

fn main() {
    let v = Volatile { value: 3u8 };
// error: field `value` of `Volatile` is private
}

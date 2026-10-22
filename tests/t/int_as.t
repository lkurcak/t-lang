// `x as T` is the same number, as a `T`. It can't panic where the checker
// proves the number fits: always, from a type into a wider one.
fn widen(b: u8) -> i64 { b as i64 }

fn to_usize(n: u32) -> usize { n as usize }

fn from_usize(n: usize) -> u64 { n as u64 }

fn narrow(n: i64) -> u8 {
    if n >= 0 && n < 256 { n as u8 } else { 0 }
}

fn wrong(n: i64) -> u8 with panic { n as u8 }

fn main() with panic {
    print(widen(200));          // expect: 200
    print(to_usize(7));         // expect: 7
    print(from_usize(8));       // expect: 8
    print(narrow(255));         // expect: 255
    print(narrow(256));         // expect: 0
    print(narrow(-1));          // expect: 0
    print(wrong(17));           // expect: 17
    let c: i8 = -1;
    print(c as i64);            // expect: -1
    // A name after `as`, so this is a comparison.
    print_bool(c as i64 < 0);   // expect: true
    print(wrong(300));
}
// error: runtime error: `as` overflowed: 300 isn't a `u8`

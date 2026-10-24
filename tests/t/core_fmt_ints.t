// Every integer type is `Display`, in decimal, down to its least value and
// up to its greatest, and none of it can panic: `show` says only `alloc`.
use alloc::string::String;
use core::fmt::Write;
use core::num::{greatest, least};

fn show<T: Int + core::fmt::Display>(out: &mut std::io::Stdout, lo: T, hi: T) -> Result<(), std::io::Error>
with alloc
{
    let mut s = String::new();
    s.put(&lo);
    s.write_char(' ');
    s.put(&hi);
    let w = out.line(&s.as_str());
    s.discard();
    w
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    show(&mut out, least::<i8>(), greatest::<i8>())?;       // expect: -128 127
    show(&mut out, least::<i16>(), greatest::<i16>())?;     // expect: -32768 32767
    show(&mut out, least::<i32>(), greatest::<i32>())?;     // expect: -2147483648 2147483647
    show(&mut out, least::<i64>(), greatest::<i64>())?;     // expect: -9223372036854775808 9223372036854775807
    show(&mut out, least::<isize>(), greatest::<isize>())?; // expect: -9223372036854775808 9223372036854775807
    show(&mut out, least::<u8>(), greatest::<u8>())?;       // expect: 0 255
    show(&mut out, least::<u16>(), greatest::<u16>())?;     // expect: 0 65535
    show(&mut out, least::<u32>(), greatest::<u32>())?;     // expect: 0 4294967295
    show(&mut out, least::<u64>(), greatest::<u64>())?;     // expect: 0 18446744073709551615
    show(&mut out, least::<usize>(), greatest::<usize>())?; // expect: 0 18446744073709551615
    show(&mut out, -7i64, 10i64)?;                          // expect: -7 10
    show(&mut out, -10i32, 9i32)?;                          // expect: -10 9
    show(&mut out, 100u8, 101u8)?;                          // expect: 100 101
    Result::Ok(())
}

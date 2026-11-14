// `{n:05}` writes integer `n` in decimal with at least 5 characters, 0s
// in front, after its sign: as a clock's `{m:02}:{s:02}` does.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (m, s) = (7u8, 5u32);
    out.line(&f"{m:02}:{s:02}")?; // expect: 07:05
    let (a, b, c) = (-42i32, 123456u64, 0i8);
    out.line(&f"{a:05}|{b:03}|{c:01}|{a:02}")?; // expect: -0042|123456|0|-42
    let least = core::num::least::<i64>();
    out.line(&f"{least:021}") // expect: -09223372036854775808
}

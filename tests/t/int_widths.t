// The ten integer types, each the integers between its least and its
// greatest. `+` and the others take two of the same type and give that
// type.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: i8 = -128;
    let b: u8 = 255;
    let c: i16 = -32768;
    let d: u16 = 65535;
    let e: i32 = -2147483647 - 1;
    let f: u32 = 4294967295;
    let g: i64 = -9223372036854775807 - 1;
    let h: u64 = 18446744073709551615;
    let i: isize = -5;
    let j: usize = 5;
    out.line(&a)?;       // expect: -128
    out.line(&b)?;       // expect: 255
    out.line(&c)?;       // expect: -32768
    out.line(&d)?;       // expect: 65535
    out.line(&e)?;       // expect: -2147483648
    out.line(&f)?;       // expect: 4294967295
    out.line(&g)?;       // expect: -9223372036854775808
    out.line(&h)?;       // expect: 18446744073709551615
    out.line(&(i * 2))?;   // expect: -10
    out.line(&(j + 1))?;   // expect: 6
    out.line(&(b - 5))?;   // expect: 250
    out.line(&(h / 3))?;   // expect: 6148914691236517205
    Result::Ok(())
}

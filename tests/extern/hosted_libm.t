// Floats cross as C's `double` and `float`, and on the hosted target the C
// math library's functions can be imported, though T's floats need none.
// Its `sqrt` and `fmaf` round once, as IEEE 754 says; not all of them do:
// glibc's `cbrt(-27.0)` is `-3.0000000000000004`.
extern fn sqrt(x: f64) -> f64;
extern fn fmaf(a: f32, b: f32, c: f32) -> f32;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    // SAFETY: `sqrt` and `fmaf` work out a number, and do nothing else.
    let (a, b) = unsafe { (sqrt(2.25), fmaf(1.5, 3.0, 0.25)) };
    out.line(&a)?;
    out.line(&b)?;
    Ok(())
}
// prints: 1.5
// prints: 4.75

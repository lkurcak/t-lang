// An intrinsic, as a function value: the C backend writes a function for
// each copy used as one (docs/33-c-backend.md).
fn apply(f: fn(&mut i64, &mut i64), a: &mut i64, b: &mut i64) { f(a, b) }

fn stop(f: fn(&str) -> ! with panic) -> i64 with panic { f("through a value") }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut a = 1i64;
    let mut b = 2i64;
    apply(core::mem::swap::<i64>, &mut a, &mut b);
    out.line(&a)?; // expect: 2
    let g: fn() -> u8 = core::num::greatest::<u8>;
    out.line(&g())?; // expect: 255
    out.line(&stop(core::panic::panic))?;
    Result::Ok(())
}
// error: runtime error: panicked: through a value

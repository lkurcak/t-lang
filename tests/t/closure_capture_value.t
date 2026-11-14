// What a closure captured by value is the value it was made with, so long
// as its body doesn't change it; one it changes still has its refinement,
// which each write keeps.
fn twice<F: FnMut() -> u32 + Discard>(mut f: F) -> u32 {
    let _ = f();
    f()
}

fn call2<F: Fn() -> u32 + Discard>(f: F, _x: ()) -> u32 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u32; 3] = [1, 2, 3];
    let i: usize = 2;
    out.line(&twice(move || a[i]))?; // expect: 3
    let mut n: { v: usize | v < 3 } = 0;
    out.line(
        &twice(move || {
            let r = a[n];
            n = 2;
            r
        }),
    )?; // expect: 3
    let mut m: usize = 1;
    out.line(
        &call2(move || a[m], {
            m = 7;
        }),
    )?;            // expect: 2
    out.line(&m)?; // expect: 7
    Ok(())
}

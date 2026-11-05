// A static is one place for the whole run, which every function can reach:
// so what's in it changes only through an atomic, and a use of it is the
// effect `global`.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn bump() -> u32 with global { COUNT.fetch_add(1) }

// Not exported, so its effects are inferred: `global` too.
fn twice() {
    let _ = bump();
    let _ = bump();
}

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    out.line(&bump())?; // expect: 0
    twice();
    out.line(&COUNT.load())?; // expect: 3
    let c: &'static Atomic<u32> = &COUNT;
    c.store(10);
    out.line(&bump())?; // expect: 10
    Ok(())
}

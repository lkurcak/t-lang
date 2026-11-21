// A function can say when it uses a static, as when it panics: a use of
// one is a site like any other (docs/90-conditional-effects.md).
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn step(counted: bool) -> u32 with global if counted {
    match counted {
        true => COUNT.fetch_add(1),
        false => 0,
    }
}

fn quiet() -> u32 with {} { step(false) }

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    out.line(&quiet())?;    // expect: 0
    out.line(&step(true))?; // expect: 0
    out.line(&step(true))?; // expect: 1
    Ok(())
}

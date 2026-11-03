// A const's initializer may use a cell of its own: only its value must be
// frozen.
use core::cell::Cell;

fn count() -> i64 with panic {
    let c = Cell::new(0i64);
    for _ in 0..3i64 {
        c.set(c.get() + 2);
    }
    c.get()
}

const SIX: i64 = count();

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&SIX)?; // expect: 6
    Result::Ok(())
}

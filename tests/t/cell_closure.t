// An `Fn` closure, called through a `&`, can change a cell it sees.
use core::cell::Cell;

fn twice<F: Fn() with panic>(f: &F) with panic { f(); f(); }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let calls = Cell::new(0i64);
    twice(&|| calls.set(calls.get() + 1));
    twice(&|| calls.set(calls.get() + 1));
    out.line(&calls.get())?; // expect: 4
    Result::Ok(())
}

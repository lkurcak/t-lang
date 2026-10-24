// A `match` moves out what its arm binds: here the box in `o`.

use alloc::boxed::Box;

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let o = Option::Some(Box::new(1i64));
    match o {
        Option::Some(b) => out.line(&(*b))?,
        Option::None => {}
    }
    // Looking at which variant it is is fine...
    match o {
        Option::Some(_) => out.line(&0i64)?,
        Option::None => out.line(&1i64)?,
    }
    // ...but not at what was moved.
    match o {
        Option::Some(b) => out.line(&(*b))?,
        Option::None => {}
    }
    Result::Ok(())
}
// error: use of moved value `o.0`

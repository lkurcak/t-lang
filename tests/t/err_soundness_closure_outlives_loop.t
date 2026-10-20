// Each time round the loop makes a closure of the same type, borrowing a
// different `n`. The one kept from an earlier round would point to an `n`
// that's gone.
#[derive(Discard)]
enum Opt<F> { None, Some(F) }

fn main() with panic {
    let mut v = Opt::None;
    let mut i = 0;
    loop {
        let n = i;
        v = Opt::Some(|| n);
        i = i + 1;
        if i == 2 { break; }
    }
    match v {
        Opt::Some(f) => print(f()),
        Opt::None => {}
    }
}
// error: `n` doesn't live long enough

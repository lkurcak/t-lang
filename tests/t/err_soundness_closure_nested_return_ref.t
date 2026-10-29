// A nested closure's inferred result lifetime belongs to that closure.
// Returning its borrow through an outer closure must report the escaping
// local, rather than leaving an unnumbered result region in the caller.
fn choose<'a>(outer: &'a i64, stop: bool) -> &'a i64 {
    let local: i64 = 1;
    let get = || { let inner = || &local; inner() };
    if stop { return get(); }
    outer
}

fn main() { let _ = choose(&0, true); }
// error: `local` doesn't live long enough

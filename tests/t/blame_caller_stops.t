// A function without the attribute ends the chain: the panic is shown at
// the call inside `middle`, which is where `middle` has to fix it.
#[blame_caller]
fn inner(n: u8) with panic {
    if n > 2 { panic("too big") }
}

fn middle(n: u8) with panic { inner(n + 1); }

#[blame_caller]
fn outer(n: u8) with panic { middle(n); }

fn main() { outer(2); }
// error: blame_caller_stops.t:8:31

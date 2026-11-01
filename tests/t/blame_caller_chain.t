// Blame goes up through every `#[blame_caller]` function: `outer` blames
// its caller too, so the panic in `inner` is shown at `outer`'s call.
#[blame_caller]
fn inner(n: u8) with panic {
    if n > 2 { panic("too big") }
}

#[blame_caller]
fn outer(n: u8) with panic { inner(n); }

fn main() {
    outer(1);
    outer(3);
}
// error: blame_caller_chain.t:13:5

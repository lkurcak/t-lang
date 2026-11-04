// An `unsafe` block keeps only an import's obligations: what it says it
// does, it does, and its caller must say so too.
extern fn may_panic() with panic;

fn quiet() with {} {
    // SAFETY: `may_panic` is the platform's, and does only what it says.
    unsafe { may_panic() }
// error: `may_panic` may panic, but `quiet` doesn't say `with panic`
}

fn main() with panic {
    quiet();
}

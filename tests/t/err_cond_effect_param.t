// An effect parameter is what the caller gives: it can't have a condition.
fn run<effect e, F: Fn() with e>(f: F, go: bool) with e if go {
    if go {
        f();
    }
}

fn main() {}
// error: only `panic`, `alloc` and `global` can have a condition

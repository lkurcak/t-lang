// A `move` closure that owns `fd` but only reads it would be callable many
// times and never give `fd` back: `fd` would be dropped with it.
struct Fd { n: i64 }

fn main() {
    let fd = Fd { n: 3 };
    let c = move || fd.n;
    print(c());
}
// error: `fd` is never used up

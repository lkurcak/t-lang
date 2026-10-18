// The closure's result borrows `n`, but `get` says it's `'static`, and
// returns it out. A closure's type is invariant in its lifetimes, so the
// `'static` reaches what it borrows. (If its signature's lifetimes were
// free to vary apart from its fields', this would be a use after free.)
fn get<F: Fn() -> &'static int + Discard>(f: F) -> &'static int { f() }

fn main() {
    let s: &int;
    {
        let n = 1;
        s = get(|| &n);
    }
    print(*s);
}
// error: `n` doesn't live long enough

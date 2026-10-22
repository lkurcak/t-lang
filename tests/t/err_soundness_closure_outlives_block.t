// A closure that borrows `n` can't outlive `n`: its type says what it
// borrows, so the borrow checker sees the reference inside it.
#[derive(Discard)]
struct Holder<F> { f: F }

fn main() {
    let h = {
        let n: i64 = 5;
        Holder { f: || n }
    };
    print((h.f)());
}
// error: `n` doesn't live long enough

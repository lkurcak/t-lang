// What `reader` returns holds `x`, so it can't outlive `n`: its type
// borrows for `'a`, and the borrow checker sees that wherever it goes.
fn reader<'a>(x: &'a int) -> impl Fn() -> int + Discard { move || *x }

fn main() {
    let r = {
        let n = 3;
        reader(&n)
    };
    print(r());
}
// error: `n` doesn't live long enough

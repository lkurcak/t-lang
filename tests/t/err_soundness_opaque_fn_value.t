// Through a function value, what `reader` returns still borrows what its
// argument does: the function type is higher-ranked, and its result type
// is the opaque type at the lifetime of each call.
fn reader(x: &int) -> impl Fn() -> int + Discard { move || *x }

fn main() {
    let g = reader;
    let f = {
        let n = 1;
        g(&n)
    };
    print(f());
}
// error: `n` doesn't live long enough

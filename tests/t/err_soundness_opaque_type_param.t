// The type `hold` returns may hold its `T`: here a `&int`, so it can't
// outlive `n`.
fn hold<T: Copy>(x: T) -> impl Fn() -> T + Discard { move || x }

fn main() {
    let f = {
        let n = 1;
        hold(&n)
    };
    print(*f());
}
// error: `n` doesn't live long enough

// The type `hold` returns may hold its `T`: here a `&i64`, so it can't
// outlive `n`.
fn hold<T: Copy>(x: T) -> impl Fn() -> T + Discard { move || x }

fn main() {
    let f = {
        let n: i64 = 1;
        hold(&n)
    };
    print(*f());
}
// error: `n` doesn't live long enough

// `pick`'s closure returns a `&'a int`, and `get` wants one that returns a
// `&'static int`: that makes `'a` `'static`, and `n` doesn't live that long.
fn pick<'a>(x: &'a int) -> impl Fn() -> &'a int + Discard { move || x }

fn get<F: Fn() -> &'static int + Discard>(f: F) -> &'static int { f() }

fn main() {
    let s: &int;
    {
        let n = 1;
        s = get(pick(&n));
    }
    print(*s);
}
// error: `n` doesn't live long enough

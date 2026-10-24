// `effect e` declares an effect parameter: the keyword says its kind, not
// its case. It goes anywhere after the lifetimes, among type parameters.
fn apply<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with e { f() }

fn pair<A, effect e, B, F: Fn(A) -> B with e + Discard>(f: F, a: A) -> B with e { f(a) }

// An impl's and a method's, and one named like a value: `f` the effect and
// `f` the variable don't meet, as they're in different places.
#[derive(Discard)]
struct Twice<F> { f: F }

impl<effect f, F: Fn(i64) -> i64 with f> Twice<F> {
    fn run(&self, x: i64) -> i64 with f {
        let f = &self.f;
        f(f(x))
    }
}

fn main() with panic {
    print(apply(|| 1)); // expect: 1
    print(pair(|x: i64| x * 2, 3)); // expect: 6
    print(Twice { f: |x: i64| x / 1 }.run(5)); // expect: 5
}

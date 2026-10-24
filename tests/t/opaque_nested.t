// `impl` can be anywhere in a return type: each is a type of its own.
enum Option<T> { Some(T), None }

fn maybe(n: i64) -> Option<impl Fn() -> i64 with panic + Discard> {
    if n > 0 { Option::Some(move || n) } else { Option::None }
}

#[derive(Discard)]
struct Pair<A, B> { a: A, b: B }

fn two(n: i64) -> Pair<impl Fn() -> i64 with panic + Discard, impl Fn() -> i64 with panic + Discard> {
    Pair { a: move || n, b: move || n + 1 }
}

// Methods can return them too, capturing the impl's parameters.
#[derive(Discard)]
struct Cell<T> { v: T }

impl<T: Copy> Cell<T> {
    fn getter(&self) -> impl Fn() -> T with panic + Discard {
        let v = self.v;
        move || v
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    match maybe(3) {
        Option::Some(f) => out.line(&f())?, // expect: 3
        Option::None => out.line(&0i64)?,
    }
    let p = two(5);
    out.line(&((p.a)() + (p.b)()))?;       // expect: 11
    let c = Cell { v: true };
    out.line(&c.getter()())?;       // expect: true
    Result::Ok(())
}

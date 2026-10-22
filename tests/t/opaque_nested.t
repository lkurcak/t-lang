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

fn main() with panic {
    match maybe(3) {
        Option::Some(f) => print(f()), // expect: 3
        Option::None => print(0i64),
    }
    let p = two(5);
    print((p.a)() + (p.b)());       // expect: 11
    let c = Cell { v: true };
    print_bool(c.getter()());       // expect: true
}

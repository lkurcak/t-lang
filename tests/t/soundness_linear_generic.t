// Linear values through generic code, traits and derived impls, which must
// run as they were checked: the interpreter counts throwing away a value
// that isn't `Discard` as a leak, and moves every value that isn't `Copy`.

struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn ignore(_n: int) -> int { 0 }

fn throw<T: Discard>(_x: T) {}

// `Discard` for every `T`: the field is a function.
#[derive(Discard)]
struct W<T> { f: fn(T) -> int }

#[allow(dead_code)]
#[derive(Discard)]
enum E<T> { A(fn(T) -> int), B }

#[derive(Discard)]
enum Option<T> { Some(T), None }

fn take<T>(o: Option<T>, k: fn(T) -> int) -> int {
    match o {
        Option::Some(x) => k(x),
        Option::None => 0,
    }
}

// A blanket impl for every `Discard` type, through a supertrait.
trait Sub: Discard {
    fn m(&self) -> int { 1 }
}

impl<T: Discard> Sub for T {}

fn drop_sub<T: Sub>(x: T) -> int { x.m() }

// A default that throws `self` away, allowed by the supertrait.
trait Eat: Discard {
    fn eat(self) -> int { 2 }
}

#[derive(Discard)]
struct D { n: int }

impl Eat for D {}

// `Copy` for every `T`, so copied even at `T` = `Fd`.
#[derive(Discard, Copy)]
struct C<T> { f: fn(T) -> int }

fn dup<T: Copy>(x: T) -> int {
    let _a = x;
    let _b = x;
    2
}

// An `eq` of a linear type borrows, so both are still there after.
impl Eq for Fd {
    fn eq(&self, o: &Fd) -> bool { self.n == o.n }
}

#[derive(Eq)]
struct P { a: Fd }

fn main() {
    throw(W { f: close });
    let _e: E<Fd> = E::B;
    print(take(Option::Some(Fd { n: 3 }), close)); // expect: 3
    print(take(Option::None, close));              // expect: 0
    print(drop_sub(3));                            // expect: 1
    print(D { n: 1 }.eat());                       // expect: 2
    print(dup(C { f: close }));                    // expect: 2
    print(dup(C { f: ignore }));                   // expect: 2
    let a = Fd { n: 1 };
    let b = Fd { n: 1 };
    print_bool(a == b);                            // expect: true
    print(close(a) + close(b));                    // expect: 2
    let x = P { a: Fd { n: 4 } };
    let y = P { a: Fd { n: 4 } };
    print_bool(x == y);                            // expect: true
    let P { a } = x;
    let P { a: c } = y;
    print(close(a) + close(c));                    // expect: 8
}

// Linear values through generic code, traits and derived impls, which must
// run as they were checked: the interpreter counts throwing away a value
// that isn't `Discard` as a leak, and moves every value that isn't `Copy`.

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn ignore(_n: i64) -> i64 { 0 }

fn throw<T: Discard>(_x: T) {}

// `Discard` for every `T`: the field is a function.
#[derive(Discard)]
struct W<T> { f: fn(T) -> i64 }

#[allow(dead_code)]
#[derive(Discard)]
enum E<T> { A(fn(T) -> i64), B }

#[derive(Discard)]
enum Option<T> { Some(T), None }

fn take<T>(o: Option<T>, k: fn(T) -> i64) -> i64 {
    match o {
        Option::Some(x) => k(x),
        Option::None => 0,
    }
}

// A blanket impl for every `Discard` type, through a supertrait.
trait Sub: Discard {
    fn m(&self) -> i64 { 1 }
}

impl<T: Discard> Sub for T {}

fn drop_sub<T: Sub>(x: T) -> i64 { x.m() }

// A default that throws `self` away, allowed by the supertrait.
trait Eat: Discard {
    fn eat(self) -> i64 { 2 }
}

#[derive(Discard)]
struct D { n: i64 }

impl Eat for D {}

// `Copy` for every `T`, so copied even at `T` = `Fd`.
#[derive(Discard, Copy)]
struct C<T> { f: fn(T) -> i64 }

fn dup<T: Copy>(x: T) -> i64 {
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

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    throw(W { f: close });
    let _e: E<Fd> = E::B;
    out.line(&take(Option::Some(Fd { n: 3 }), close))?; // expect: 3
    out.line(&take(Option::None, close))?;              // expect: 0
    out.line(&drop_sub(3i64))?;                            // expect: 1
    out.line(&(D { n: 1 }.eat()))?;                       // expect: 2
    out.line(&dup(C { f: close }))?;                    // expect: 2
    out.line(&dup(C { f: ignore }))?;                   // expect: 2
    let a = Fd { n: 1 };
    let b = Fd { n: 1 };
    let same = a == b;
    let sum = close(a) + close(b);
    out.line(&same)?;                                   // expect: true
    out.line(&sum)?;                                    // expect: 2
    let x = P { a: Fd { n: 4 } };
    let y = P { a: Fd { n: 4 } };
    let same = x == y;
    let P { a } = x;
    let P { a: c } = y;
    let sum = close(a) + close(c);
    out.line(&same)?;                                   // expect: true
    out.line(&sum)                                      // expect: 8
}

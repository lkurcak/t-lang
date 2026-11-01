// `==` on what isn't a number, a `bool` or a character: field by field for
// a derived `Eq`, by its own `eq` where a type has one, through references
// and boxes, value by value for arrays, slices and strings. Each is a
// function monomorphisation writes out for the type (docs/32.1).
use alloc::boxed::Box;

/// Equal whatever the numbers, if they're both even or both odd.
#[derive(Discard)]
struct Parity { n: i64 }

impl Eq for Parity {
    fn eq(&self, other: &Parity) -> bool { self.n % 2 == other.n % 2 }
}

#[derive(Discard, Eq)]
struct Pair { p: Parity, name: &'static str }

#[allow(dead_code)]
#[derive(Discard, Eq)]
enum Shape { Dot, Line(i64), Box(Box<Pair>, bool) }

fn same<T: Eq>(a: &T, b: &T) -> bool { a == b }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = Pair { p: Parity { n: 1 }, name: "a" };
    let b = Pair { p: Parity { n: 3 }, name: "a" };
    let c = Pair { p: Parity { n: 2 }, name: "a" };
    out.line(&(a == b))?; // expect: true
    out.line(&(a == c))?; // expect: false
    out.line(&(a != c))?; // expect: true
    out.line(&(&&a == &&b))?; // expect: true
    out.line(&same(&a, &b))?; // expect: true

    let s = Shape::Box(Box::new(Pair { p: Parity { n: 4 }, name: "x" }), true);
    let t = Shape::Box(Box::new(Pair { p: Parity { n: 6 }, name: "x" }), true);
    let u = Shape::Box(Box::new(Pair { p: Parity { n: 6 }, name: "y" }), true);
    out.line(&(s == t))?; // expect: true
    out.line(&(s == u))?; // expect: false
    out.line(&(Shape::Line(1) == Shape::Line(1)))?; // expect: true
    out.line(&(Shape::Line(1) == Shape::Dot))?; // expect: false
    out.line(&(Shape::Dot == Shape::Dot))?; // expect: true

    let xs = [Parity { n: 1 }, Parity { n: 2 }];
    let ys = [Parity { n: 3 }, Parity { n: 4 }];
    let zs = [Parity { n: 3 }, Parity { n: 5 }];
    out.line(&(xs == ys))?; // expect: true
    out.line(&(xs == zs))?; // expect: false
    out.line(&(xs.as_slice() == ys.as_slice()))?; // expect: true
    match ys.as_slice().split_at(1) {
        Option::Some(core::slice::Split { head, .. }) => out.line(&(xs.as_slice() == head))?, // expect: false
        Option::None => {}
    }
    out.line(&("abc" == "abc"))?; // expect: true
    out.line(&("abc" == "abd"))?; // expect: false
    out.line(&("ab" == "abc"))?; // expect: false
    out.line(&(Box::new(5i64) == Box::new(5i64)))?; // expect: true
    Result::Ok(())
}

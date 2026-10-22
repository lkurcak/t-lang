// `where` clauses: bounds after a signature, on a type parameter, on
// `Self`, or on a projection. They mean what bounds in `<..>` would.

trait Source {
    type Out;

    fn get(&mut self) -> Self::Out with panic;

    // Only for a source that may be thrown away, and whose items may be.
    fn skip(mut self, n: i64) -> i64 with panic where Self: Discard, Self::Out: Discard {
        let mut i = 0;
        while i < n {
            let _x = self.get();
            i = i + 1;
        }
        n
    }
}

#[derive(Discard)]
struct Nat { n: i64 }

impl Source for Nat {
    type Out = i64;

    fn get(&mut self) -> i64 with panic {
        self.n = self.n + 1;
        self.n
    }

    // An impl's version says the same as the trait's.
    fn skip(mut self, n: i64) -> i64 with panic where Self: Discard, Self::Out: Discard {
        self.n = self.n + n;
        self.n
    }
}

fn pick<T>(a: T, b: T, first: bool) -> T where T: Discard {
    if first { a } else { b }
}

fn apply<e, F>(f: F, x: i64) -> i64 with e where F: Fn(i64) -> i64 with e + Discard {
    f(x)
}

fn main() with panic {
    print(Nat { n: 0 }.skip(5)); // expect: 5
    print(pick(1i64, 2, false));    // expect: 2
    print(apply(|x| x * 2, 21)); // expect: 42
}

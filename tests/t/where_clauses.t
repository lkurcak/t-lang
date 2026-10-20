// `where` clauses: bounds after a signature, on a type parameter, on
// `Self`, or on a projection. They mean what bounds in `<..>` would.

trait Source {
    type Out;

    fn get(&mut self) -> Self::Out;

    // Only for a source that may be thrown away, and whose items may be.
    fn skip(mut self, n: int) -> int where Self: Discard, Self::Out: Discard {
        let mut i = 0;
        while i < n {
            let _x = self.get();
            i = i + 1;
        }
        n
    }
}

#[derive(Discard)]
struct Nat { n: int }

impl Source for Nat {
    type Out = int;

    fn get(&mut self) -> int {
        self.n = self.n + 1;
        self.n
    }

    // An impl's version says the same as the trait's.
    fn skip(mut self, n: int) -> int where Self: Discard, Self::Out: Discard {
        self.n = self.n + n;
        self.n
    }
}

fn pick<T>(a: T, b: T, first: bool) -> T where T: Discard {
    if first { a } else { b }
}

fn apply<F>(f: F, x: int) -> int where F: Fn(int) -> int + Discard {
    f(x)
}

fn main() {
    print(Nat { n: 0 }.skip(5)); // expect: 5
    print(pick(1, 2, false));    // expect: 2
    print(apply(|x| x * 2, 21)); // expect: 42
}

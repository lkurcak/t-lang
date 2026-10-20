// Generic code runs as it was checked, whatever its type parameters turn
// out to be. Here each `S`-like struct isn't `Discard`, and each method
// uses it up by moving its field out, of a type parameter that isn't known
// to be `Copy`. At `int`, or at a closure that only borrows, the field is
// `Copy`, but its use is still the move the checker saw: the struct is
// used up, not left behind to leak.

fn app<F: Discard>(_f: F) -> bool { true }

fn first<T: Discard>(x: T, _y: T) -> T { x }

trait Use {
    fn use_(self) -> int;
}

impl Use for int {
    fn use_(self) -> int { self + 1 }
}

#[derive(Copy, Discard)]
struct In<P> { p: P }

struct S<P> { p: P }

impl<P: Discard> S<P> {
    // As an argument.
    fn arg(self) -> bool { app(self.p) }

    // Into a variable.
    fn bind(self) -> P {
        let q = self.p;
        q
    }

    // Twice in one call, from two structs.
    fn pick(self, other: S<P>) -> P { first(self.p, other.p) }
}

impl<P: Use> S<P> {
    // As the receiver of a method that takes `self`.
    fn recv(self) -> int { self.p.use_() }
}

struct Out<P> { i: In<P> }

impl<P: Discard> Out<P> {
    // Matched by value: `In<P>` is `Copy` only if `P` is.
    fn take(self) -> P {
        match self.i {
            In { p } => p,
        }
    }
}

struct K<F> { f: F }

impl<F: FnOnce() -> int> K<F> {
    // Called, which takes it, as an `FnOnce`.
    fn call(self) -> int { (self.f)() }
}

fn main() {
    print_bool(S { p: 1 }.arg());                // expect: true
    print(S { p: 2 }.bind());                    // expect: 2
    print(S { p: 3 }.pick(S { p: 4 }));          // expect: 3
    print(S { p: 4 }.recv());                    // expect: 5
    print(Out { i: In { p: 6 } }.take());        // expect: 6
    let n = 7;
    print(K { f: || n }.call());                 // expect: 7
    // A closure that only borrows is `Copy` too.
    print_bool(S { p: || n }.arg());             // expect: true
}

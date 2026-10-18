// Bounds with associated types: equality bounds on impls and structs as
// well as functions, bounds on projections, and `==` on a projection.

trait Tr {
    type Out;

    fn out(&self) -> Self::Out;
}

#[derive(Discard)]
struct A { n: int }

impl Tr for A {
    type Out = int;

    fn out(&self) -> int { self.n }
}

#[derive(Discard)]
struct B { b: bool }

impl Tr for B {
    type Out = bool;

    fn out(&self) -> bool { self.b }
}

trait Show {
    fn show(&self) -> int;
}

#[derive(Discard)]
struct W<T> { t: T }

// Only for a `T` whose `Out` is an `int`.
impl<T: Tr<Out = int>> Show for W<T> {
    fn show(&self) -> int { self.t.out() * 10 }
}

fn shown<S: Show>(s: &S) -> int { s.show() }

// A struct that's only for such a `T`, whose field is an `int`.
#[derive(Discard)]
struct Count<T: Tr<Out = int>> { of: T, n: T::Out }

fn count<T: Tr<Out = int>>(of: T) -> Count<T> {
    let n = of.out() + 1;
    Count { of, n }
}

// `==` on what a projection stands for, by a bound on it.
fn same<T: Tr<Out: Eq + Discard>>(x: &T, y: &T) -> bool { x.out() == y.out() }

// A rigid projection outlives `'r` when its type does.
#[derive(Discard)]
struct Hold<'r, U: 'r> { u: U, r: &'r int }

fn hold<'r, T: Tr<Out: Discard> + 'r>(t: &T, r: &'r int) -> int {
    let h: Hold<'r, T::Out> = Hold { u: t.out(), r };
    *h.r
}

fn main() {
    print(shown(&W { t: A { n: 4 } })); // expect: 40
    let c = count(A { n: 1 });
    print(c.n + c.of.n);                               // expect: 3
    print_bool(same(&A { n: 1 }, &A { n: 1 }));        // expect: true
    print_bool(same(&B { b: true }, &B { b: false })); // expect: false
    print(hold(&A { n: 1 }, &5));                      // expect: 5
}

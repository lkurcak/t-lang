// `==` is `Eq::eq(&l, &r)`, so a type with an `eq` of its own compares by
// it, wherever the `==` is: in generic code, in a derived `eq` comparing a
// field, through references and boxes. (It used to compare field by field
// unless something called that `eq` by name.)

#[derive(Discard)]
struct S { n: i64 }

// Every `S` is the same as every other.
impl Eq for S {
    fn eq(&self, _o: &S) -> bool { true }
}

#[derive(Discard, Eq)]
struct W<T> { x: T }

#[derive(Discard)]
struct Tagged<T> { x: T, tag: i64 }

impl<T> Eq for Tagged<T> {
    fn eq(&self, o: &Tagged<T>) -> bool { self.tag == o.tag }
}

#[derive(Discard)]
enum E { A(i64), B }

impl Eq for E {
    fn eq(&self, _o: &E) -> bool { true }
}

fn same<T: Eq>(a: &T, b: &T) -> bool { a == b }

fn main() with {alloc, panic} {
    print_bool(S { n: 1 } == S { n: 2 });                     // expect: true
    print_bool(same(&S { n: 1 }, &S { n: 2 }));               // expect: true
    print_bool(W { x: S { n: 1 } } == W { x: S { n: 2 } });   // expect: true
    print_bool(Tagged { x: 1i64, tag: 1 } == Tagged { x: 2, tag: 1 }); // expect: true
    print_bool(Tagged { x: 1i64, tag: 1 } != Tagged { x: 1, tag: 2 }); // expect: true
    print_bool(E::A(1) == E::B);                              // expect: true
    print_bool(Box::new(S { n: 1 }) == Box::new(S { n: 2 })); // expect: true
    let a = S { n: 1 };
    let b = S { n: 3 };
    print_bool(&a == &b);                                     // expect: true
    // As a function: the derived `eq` of `W`, which is `==`.
    let f = W::<S>::eq;
    print_bool(f(&W { x: S { n: 1 } }, &W { x: S { n: 4 } })); // expect: true
    print_bool(Eq::eq(&1i64, &2));                               // expect: false
}

// A closure whose parameter is annotated `&i64` takes a reference of every
// lifetime, so it meets a bound that says so, `P: Fn(&i64) -> bool`, even
// where that bound is an impl's and the closure was made before it was
// asked. The bound and the closure's type agree, up to the names of the
// lifetimes they're for.
trait Check {
    fn passes(&self, x: &i64) -> bool;
}

#[derive(Discard where P: Discard)]
struct Test<P> {
    p: P,
}

impl<P: Fn(&i64) -> bool> Check for Test<P> {
    fn passes(&self, x: &i64) -> bool {
        (self.p)(x)
    }
}

fn check_all<C: Check>(c: &C, a: &i64, b: &i64) -> bool {
    c.passes(a) && c.passes(b)
}

fn main() {
    let positive = Test { p: |x: &i64| *x > 0 };
    let a = 1;
    print_bool(positive.passes(&a));                // expect: true
    let low = 5;
    let above = Test { p: |x: &i64| *x > low };
    let b = 9;
    print_bool(above.passes(&b));                   // expect: true
    print_bool(check_all(&above, &a, &b));          // expect: false
    print_bool(check_all(&positive, &a, &b));       // expect: true
}

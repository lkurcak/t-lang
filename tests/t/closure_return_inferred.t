// What a closure's result borrows from, its body says: one of its
// parameters, whose lifetime each call picks, or what it captured. A
// lifetime left out in a written result means the same: one to work out.
#[derive(Discard where P: Discard)]
struct Wrap<P> { p: P }

trait Get {
    fn get<'a>(&self, x: &'a i64) -> Option<&'a i64>;
}

// Needs a closure that returns what lives as long as its argument.
impl<P: Fn(&i64) -> Option<&i64>> Get for Wrap<P> {
    fn get<'a>(&self, x: &'a i64) -> Option<&'a i64> { (self.p)(x) }
}

fn main() {
    let a = 1;
    // The parameter's: called with references that live for different
    // lengths of time.
    let id = |x: &i64| x;
    let long = id(&a);
    {
        let b = 2;
        print(*id(&b)); // expect: 2
    }
    print(*long); // expect: 1
    let first = |x: &i64, _y: &i64| -> &i64 { x };
    {
        let c = 3;
        print(*first(&a, &c)); // expect: 1
    }

    // Of two lifetimes in one parameter, the one it borrows from.
    let inner = |x: &&i64| *x;
    let ra = &a;
    let got;
    {
        let rra = &ra;
        got = inner(rra);
    }
    print(*got); // expect: 1
    // A parameter without an annotation takes one lifetime, worked out
    // from its uses, as before.
    let second = |_x, y: &i64| y;
    {
        let e: i64 = 6;
        print(*second(1i64, &e)); // expect: 6
    }

    // What it captured, written or not.
    let n = 5;
    let f = |_x: &i64| &n;
    let g = |_x: &i64| -> &i64 { &n };
    let r;
    {
        let d = 4;
        r = g(&d);
        print(*f(&d)); // expect: 5
    }
    print(*r); // expect: 5

    // Nothing: as for a function, the one parameter's.
    let never = Wrap { p: |_x: &i64| -> Option<&i64> { Option::None } };
    print_bool(never.get(&a).is_some()); // expect: false
    let some = Wrap { p: |x: &i64| Option::Some(x) };
    match some.get(&a) {
        Option::Some(x) => print(*x), // expect: 1
        Option::None => {}
    }
}

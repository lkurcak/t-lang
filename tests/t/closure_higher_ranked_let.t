// A closure's annotations say what it takes as a function's parameters
// would: `|x: &int|` takes a reference of any lifetime, picked at each call,
// and a written `-> &int` returns one as long-lived as that. So one closure
// can be called with references that live for different lengths of time.
fn twice(f: fn(&int) -> &int) -> int {
    let a = 1;
    let r = f(&a);
    let b = 2;
    *r + *f(&b)
}

fn main() {
    let id = |x: &int| -> &int { x };
    let a = 3;
    let long = id(&a);
    {
        let b = 4;
        print(*id(&b));             // expect: 4
    }
    print(*long);                   // expect: 3
    print(twice(id));               // expect: 3

    // One that captures, and one that changes what it captures.
    let k = 10;
    let pick = |x: &int| -> &int { if k > 0 { x } else { x } };
    {
        let c = 5;
        print(*pick(&c));           // expect: 5
    }
    print(*pick(&a));               // expect: 3
    let mut calls = 0;
    let mut count = |x: &int| -> &int {
        calls += 1;
        x
    };
    {
        let d = 6;
        print(*count(&d));          // expect: 6
    }
    print(*count(&a));              // expect: 3
    print(calls);                   // expect: 2

    // A closure in a closure: each has a lifetime of its own.
    let outer = |x: &int| -> int {
        let inner = |y: &int| -> &int { y };
        let e = 7;
        *inner(x) + *inner(&e)
    };
    print(outer(&a));               // expect: 10

    // One that captures a parameter of another's, of a lifetime that
    // closure is generic over, and returns it.
    let through = |x: &int| -> &int {
        let get = || x;
        get()
    };
    {
        let f = 8;
        print(*through(&f));        // expect: 8
    }
    print(*through(&a));            // expect: 3
}

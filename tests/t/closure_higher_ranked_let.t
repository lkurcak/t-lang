// A closure's annotations say what it takes as a function's parameters
// would: `|x: &i64|` takes a reference of any lifetime, picked at each call,
// and a written `-> &i64` returns one as long-lived as that. So one closure
// can be called with references that live for different lengths of time.
fn twice(f: fn(&i64) -> &i64) -> i64 with panic {
    let a = 1;
    let r = f(&a);
    let b = 2;
    *r + *f(&b)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let id = |x: &i64| -> &i64 { x };
    let a = 3;
    let long = id(&a);
    {
        let b = 4;
        out.line(&(*id(&b)))?;             // expect: 4
    }
    out.line(&(*long))?;                   // expect: 3
    out.line(&twice(id))?;               // expect: 3

    // One that captures, and one that changes what it captures.
    let k: i64 = 10;
    let pick = |x: &i64| -> &i64 { if k > 0 { x } else { x } };
    {
        let c = 5;
        out.line(&(*pick(&c)))?;           // expect: 5
    }
    out.line(&(*pick(&a)))?;               // expect: 3
    let mut calls: i64 = 0;
    let mut count = |x: &i64| -> &i64 {
        calls += 1;
        x
    };
    {
        let d = 6;
        out.line(&(*count(&d)))?;          // expect: 6
    }
    out.line(&(*count(&a)))?;              // expect: 3
    out.line(&calls)?;                   // expect: 2

    // A closure in a closure: each has a lifetime of its own.
    let outer = |x: &i64| -> i64 {
        let inner = |y: &i64| -> &i64 { y };
        let e = 7;
        *inner(x) + *inner(&e)
    };
    out.line(&outer(&a))?;               // expect: 10

    // One that captures a parameter of another's, of a lifetime that
    // closure is generic over, and returns it.
    let through = |x: &i64| -> &i64 {
        let get = || x;
        get()
    };
    {
        let f = 8;
        out.line(&(*through(&f)))?;        // expect: 8
    }
    out.line(&(*through(&a)))?;            // expect: 3
    Result::Ok(())
}

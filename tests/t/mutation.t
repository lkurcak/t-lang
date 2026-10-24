// `let mut` variables can be changed, by assigning to them, to their
// fields, or through a `&mut` to them.

#[derive(Discard)]
struct Counter { count: i64, step: i64 }

fn tick(c: &mut Counter) with panic { c.count += c.step; }

fn exchange(a: &mut i64, b: &mut i64) {
    let t = *a;
    *a = *b;
    *b = t;
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut x: i64 = 1;
    x = x + 1;
    x *= 10;
    out.line(&x)?;                   // expect: 20
    let mut c = Counter { count: 0, step: 3 };
    tick(&mut c);
    tick(&mut c);
    c.step = 1;
    tick(&mut c);
    out.line(&c.count)?;             // expect: 7
    let mut a = 1;
    let mut b = 2;
    exchange(&mut a, &mut b);
    out.line(&a)?;                   // expect: 2
    out.line(&b)?;                   // expect: 1
    let r = &mut a;
    *r -= 5;
    out.line(&a)?;                   // expect: -3
    Result::Ok(())
}

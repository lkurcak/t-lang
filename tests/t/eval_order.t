// Operands are worked out left to right, each to its value before the
// next: a later one that changes a variable doesn't change what an earlier
// one gave (docs/91-evaluation-order.md).
use alloc::list::List;

#[derive(Copy, Discard, Eq)]
struct P { a: u8, b: u8 }

fn first(a: u8, _b: u8) -> { r: u8 | r == a } { a }

fn bump(c: &mut u8) -> u8 { *c = 9; 0 }

fn one(_x: u8) -> u8 { 1 }

fn two(_x: u8) -> u8 { 2 }

fn need5(x: { x: u8 | x == 5 }) -> u8 { x }

impl P {
    fn sum(self, more: u8) -> u8 with panic { self.a + self.b + more }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    // A call's arguments.
    let mut c = 5u8;
    out.line(&first(c, { c = 7; 7 }))?; // expect: 5
    out.line(&c)?;                      // expect: 7
    let mut d = 1u8;
    out.line(&first(d, bump(&mut d)))?; // expect: 1
    // An operator's.
    let mut e = 5u8;
    out.line(&(e + { e = 7; 1 }))?;     // expect: 6
    let mut f = 5u8;
    out.line(&(f < { f = 7; 6 }))?;     // expect: true
    let mut g = 1u8;
    out.line(&(g + bump(&mut g)))?;     // expect: 1
    let mut p = P { a: 1, b: 1 };
    let q = P { a: 1, b: 1 };
    out.line(&(p == { p = P { a: 2, b: 2 }; q }))?; // expect: true
    // A tuple's, a struct's, an array's and a variant's.
    let mut h = 5u8;
    let t = (h, { h = 7; 7u8 });
    out.line(&t.0)?;                    // expect: 5
    let mut i = 5u8;
    let s = P { a: i, b: { i = 7; 7 } };
    out.line(&s.a)?;                    // expect: 5
    let mut j = 5u8;
    let arr = [j, { j = 7; 7u8 }];
    out.line(&arr[0])?;                 // expect: 5
    let mut k = 5u8;
    let o = Some((k, { k = 7; 7u8 }));
    out.line(&o.unwrap().0)?;           // expect: 5
    // An index.
    let a3 = [1u8, 2, 3];
    let mut n = 0usize;
    out.line(&first(a3[n], { n = 2; 0 }))?; // expect: 1
    // What's called, and a method's receiver.
    let mut fun: fn(u8) -> u8 = one;
    out.line(&fun({ fun = two; 0 }))?;  // expect: 1
    let mut r = P { a: 1, b: 1 };
    out.line(&r.sum({ r = P { a: 5, b: 5 }; 0 }))?; // expect: 2
    let mut m = 5u8;
    let cl = |a: u8, _b: u8| a;
    out.line(&cl(m, { m = 7; 7 }))?;    // expect: 5
    // A field of something a later argument takes.
    let mut l: List<u8> = List::new();
    l.push(3);
    let pair = (4u8, l);
    out.line(&first(pair.0, eat(pair)))?; // expect: 4
    let whole = (6u8, List::new());
    out.line(&both(whole.0, whole))?;     // expect: 12
    // What's behind a `&mut`, read before the reference is given away.
    let mut under = 5u8;
    let re = &mut under;
    let held = (*re, re);
    *held.1 = 9;
    out.line(&held.0)?;                 // expect: 5
    // The checker reads them the same way.
    let mut v = 5u8;
    out.line(&need5(first(v, { v = 7; 7 })))?; // expect: 5
    // `a += b` works `b` out first, then is `a = a + b`.
    let mut w = 5u8;
    w += { w = 7; 1 };
    out.line(&w)?;                      // expect: 8
    // A borrow that's passed is taken when the call is made, of what's
    // there then (docs/06-traits.md).
    let mut x = 5u8;
    out.line(&deref(&x, { x = 7; 7 }))?; // expect: 7
    Ok(())
}

fn eat(p: (u8, List<u8>)) -> u8 {
    let (a, l) = p;
    l.discard();
    a
}

fn both(a: u8, p: (u8, List<u8>)) -> u8 with panic { a + eat(p) }

fn deref(a: &u8, _b: u8) -> u8 { *a }

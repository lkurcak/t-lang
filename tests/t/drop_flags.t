// Values that are moved out on some paths and not on others: what's left
// is dropped where it goes out of scope, exactly once, by a *drop flag*
// set where it gets its value and cleared where it's moved (docs/32.1).
// The interpreter fails on a box that's never freed or freed twice, and on
// a drop the flags got wrong.
use alloc::boxed::Box;

mod noisy {
    /// Says its `id` when it's dropped whole.
    pub struct Noisy<'a> {
        pub out: &'a mut std::io::Stdout,
        pub id: i64,
        pub b: alloc::boxed::Box<i64>,
    }

    impl<'a> Discard for Noisy<'a> {
        fn discard(self) {
            let _ = self.out.line(&self.id);
        }
    }

    /// Takes `n` apart on one path only: whole, it says its id at the end.
    pub fn maybe_open(n: Noisy, c: bool) -> i64 {
        if c {
            let b = n.b;
            return *b;
        }
        0
    }

    /// Moves a field out on one path, then the scope ends: whole on the
    /// other path, so its `discard` runs there, and only there.
    pub fn maybe_part(n: Noisy, c: bool) -> i64 {
        let mut k = 0;
        if c {
            let b = n.b;
            k = *b;
        }
        k
    }
}

use noisy::Noisy;

#[derive(Discard)]
struct Two { a: Box<i64>, b: Box<i64> }

#[allow(dead_code)]
#[derive(Discard)]
enum Opt { Some(Box<i64>), None }

fn sink(b: Box<i64>) -> i64 { *b }

fn two(n: i64) -> Two with {alloc, panic} { Two { a: Box::new(n), b: Box::new(n + 1) } }

// Moved on one branch: dropped at the end on the other.
fn maybe(c: bool) -> i64 with alloc {
    let b = Box::new(1i64);
    let mut n = 0;
    if c {
        n = sink(b);
    }
    n
}

// Given a value on one branch only.
fn deferred(c: bool) -> i64 with alloc {
    let b;
    if c {
        b = Box::new(2i64);
        return *b;
    }
    0
}

// One field moved on one branch: the other is always dropped.
fn partly(c: bool) -> i64 with {alloc, panic} {
    let t = two(3);
    let mut n = 0;
    if c {
        let a = t.a;
        n = *a;
    }
    n
}

// What an enum's variant holds moved on one branch: the variant decides
// what's left, as it may not be the one moved out of.
fn variant(o: Opt, c: bool) -> i64 {
    let mut n = 0;
    if c {
        match o {
            Opt::Some(b) => n = sink(b),
            Opt::None => {},
        }
    }
    n
}

// What a box points to moved out: the box is freed, and what's left of
// its value dropped.
fn boxed(c: bool) -> i64 with {alloc, panic} {
    let b = Box::new(two(5));
    let mut n = 0;
    if c {
        let a = (*b).a;
        n = *a;
    }
    n
}

// Assigning to what may have been moved out drops the old value only if
// it's there.
fn assign(c: bool) -> i64 with alloc {
    let mut x = Box::new(7i64);
    if c {
        let _ = sink(x);
    }
    x = Box::new(8i64);
    *x
}

// A loop that moves on some rounds and gives a value back on the others.
fn rounds() -> i64 with {alloc, panic} {
    let mut x = Box::new(0i64);
    let mut i = 0;
    let mut total = 0;
    while i < 6 {
        if i % 2 == 0 {
            total += sink(x);
            x = Box::new(i);
        } else {
            x = Box::new(i * 10);
        }
        i += 1;
    }
    total + *x
}

// A value of an array moved out by a pattern on one branch.
fn array(c: bool) -> i64 with alloc {
    let a = [Box::new(1i64), Box::new(2i64), Box::new(3i64)];
    let mut n = 0;
    if c {
        let [_, b, _] = a;
        n = *b;
    }
    n
}

// A closure that uses up what it captured on one path only.
fn closure(c: bool) -> i64 with alloc {
    let b = Box::new(9i64);
    let f = || if c { sink(b) } else { 0 };
    f()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    out.line(&maybe(true))?;                            // expect: 1
    out.line(&maybe(false))?;                           // expect: 0
    out.line(&deferred(true))?;                         // expect: 2
    out.line(&deferred(false))?;                        // expect: 0
    out.line(&partly(true))?;                           // expect: 3
    out.line(&partly(false))?;                          // expect: 0
    out.line(&variant(Opt::Some(Box::new(4)), true))?;  // expect: 4
    out.line(&variant(Opt::Some(Box::new(4)), false))?; // expect: 0
    out.line(&variant(Opt::None, true))?;               // expect: 0
    out.line(&boxed(true))?;                            // expect: 5
    out.line(&boxed(false))?;                           // expect: 0
    out.line(&assign(true))?;                           // expect: 8
    out.line(&assign(false))?;                          // expect: 8
    out.line(&rounds())?;                               // expect: 90
    out.line(&array(true))?;                            // expect: 2
    out.line(&array(false))?;                           // expect: 0
    out.line(&closure(true))?;                          // expect: 9
    out.line(&closure(false))?;                         // expect: 0
    let n = Noisy { out: &mut out, id: 10, b: Box::new(11) };
    let k = noisy::maybe_open(n, true);
    out.line(&k)?; // expect: 11
    let n = Noisy { out: &mut out, id: 12, b: Box::new(13) };
    let k = noisy::maybe_open(n, false); // expect: 12
    out.line(&k)?;                       // expect: 0
    let n = Noisy { out: &mut out, id: 14, b: Box::new(15) };
    let k = noisy::maybe_part(n, true);
    out.line(&k)?; // expect: 15
    let n = Noisy { out: &mut out, id: 16, b: Box::new(17) };
    let k = noisy::maybe_part(n, false); // expect: 16
    out.line(&k)?;                       // expect: 0
    Result::Ok(())
}

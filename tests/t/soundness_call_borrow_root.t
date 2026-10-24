// A call's borrows are taken after its arguments, but only the borrow, and
// following the place's fields and `*`s, waits: what the place starts from
// is worked out where it's written. `&*make()` calls `make` in order, so an
// argument after it can't change what it saw. (The interpreter used to
// wait with all of `*{ .. }`, and so ran the move of `a` below first.)

use alloc::boxed::Box;

#[derive(Discard)]
struct S { n: i64 }

impl S {
    fn get(&self, k: Box<i64>) -> i64 with panic { self.n + *k }
}

fn f(x: &i64, y: Box<i64>) -> i64 with panic { *x + *y }

fn g(x: &mut Box<i64>, n: i64) -> i64 with panic { **x + n }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut a = Box::new(1);
    let c = a;
    out.line(&f(&*{ a = Box::new(5); Box::new(7) }, a))?; // expect: 12
    out.line(&(*c))?;                                       // expect: 1

    // The same through a method's receiver: `&*{ .. }` once it's lifted.
    let mut a = Box::new(1);
    let c = a;
    out.line(&{ a = Box::new(5); Box::new(S { n: 7 }) }.get(a))?; // expect: 12
    out.line(&(*c))?;                                              // expect: 1

    // A place whose root is a variable is followed when the borrow is
    // taken, so after `r` is changed.
    let mut x = Box::new(1);
    let mut y = Box::new(2);
    let mut r = &mut x;
    out.line(&g(&mut *r, { r = &mut y; 10 }))?; // expect: 12
    out.line(&(*x + *y))?;                        // expect: 3
    Result::Ok(())
}

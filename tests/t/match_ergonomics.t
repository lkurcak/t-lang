// Matching on a reference looks through it, and binds variables by
// reference: to what's inside the value it points to.

use alloc::boxed::Box;

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

#[derive(Discard)]
struct Wrapper { inner: Option<Box<i64>> }

fn peek(o: &Option<Box<i64>>) -> i64 {
    match o {
        Option::Some(b) => **b, // `b` is a `&Box<i64>`
        Option::None => 0,
    }
}

fn bump(o: &mut Option<i64>) with panic {
    match o {
        Option::Some(n) => *n += 1, // `n` is a `&mut i64`
        Option::None => {}
    }
}

fn as_ref<T>(o: &Option<T>) -> Option<&T> {
    match o {
        Option::Some(x) => Option::Some(x),
        Option::None => Option::None,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut stdout = os.stdout;
    let o = Option::Some(Box::new(7));
    stdout.line(&peek(&o))?;            // expect: 7
    // Nothing was moved out of `o`, so again:
    stdout.line(&peek(&o))?;            // expect: 7
    let mut n = Option::Some(1);
    bump(&mut n);
    bump(&mut n);
    match n {
        Option::Some(v) => stdout.line(&v)?, // expect: 3
        Option::None => {}
    }
    let w = Wrapper { inner: Option::Some(Box::new(9)) };
    match &w.inner {
        Option::Some(b) => stdout.line(&(**b))?, // expect: 9
        Option::None => {}
    }
    match as_ref(&w.inner) {
        Option::Some(b) => stdout.line(&(**b))?, // expect: 9
        Option::None => {}
    }
    let Wrapper { inner } = &w;
    stdout.line(&peek(inner))?;         // expect: 9
    Result::Ok(())
}

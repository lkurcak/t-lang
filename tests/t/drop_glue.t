// What drop elaboration and glue look like: `t mono` shows a drop flag
// guarding a drop that happens on one path only, an open drop of what's
// left of a struct, and the glue that drops a struct and compares one
// (see drop_glue.mono).
use alloc::boxed::Box;

#[derive(Discard, Eq)]
struct Two { a: Box<i64>, b: i64 }

fn sink(t: Two) -> i64 { t.b }

fn pick(c: bool) -> i64 with alloc {
    let t = Two { a: Box::new(1), b: 2 };
    let mut n = 0;
    if c {
        n = sink(t);
    }
    n
}

fn part(t: Two) -> i64 {
    let a = t.a;
    *a
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&pick(true))?;                         // expect: 2
    out.line(&pick(false))?;                        // expect: 0
    out.line(&part(Two { a: Box::new(3), b: 4 }))?; // expect: 3
    let same = Two { a: Box::new(5), b: 6 } == Two { a: Box::new(5), b: 6 };
    out.line(&same)?; // expect: true
    Result::Ok(())
}

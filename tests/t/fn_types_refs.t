// A function type with references in it takes references of any lifetime:
// `fn(&i64) -> i64` is a function that works for every lifetime, like a
// generic function, and each call picks one.

fn apply(f: fn(&i64) -> i64 with panic, x: &i64) -> i64 with panic { f(x) }

fn twice(f: fn(&i64) -> &i64 with panic, x: &i64) -> i64 with panic { *f(f(x)) }

fn same(r: &i64) -> &i64 { r }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let n = 3;
    out.line(&apply(|r| *r + 1, &n))?;   // expect: 4
    out.line(&twice(same, &n))?;         // expect: 3
    out.line(&twice(|r| r, &n))?;        // expect: 3
    let g: fn(&i64) -> i64 with panic = |r| *r * 10;
    let a = 1;
    let b = 2;
    out.line(&(g(&a) + g(&b)))?;           // expect: 30
    Result::Ok(())
}

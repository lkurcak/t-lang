// What effects allow: a function that can't panic, a closure's effects
// passed on through an effect parameter, and a function type gaining an
// effect.
fn max(a: i64, b: i64) -> i64 { if a < b { b } else { a } }

fn apply<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(x) }

fn inc(n: i64) -> i64 with panic { n + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    // `apply` can't panic when its closure can't.
    out.line(&apply(|x| max(x, 3), 1))?; // expect: 3
    out.line(&apply(|x| x + 1, 1))?; // expect: 2

    let f: fn(i64, i64) -> i64 with panic = max;
    out.line(&f(4, 5))?; // expect: 5
    let g: fn(i64) -> i64 with panic = inc;
    out.line(&g(4))?; // expect: 5
    Result::Ok(())
}

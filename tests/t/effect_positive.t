// What effects allow: a function that can't panic, a closure's effects
// passed on through an effect parameter, and a function type gaining an
// effect.
fn max(a: i64, b: i64) -> i64 { if a < b { b } else { a } }

fn apply<e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(x) }

fn inc(n: i64) -> i64 with panic { n + 1 }

fn main() with panic {
    // `apply` can't panic when its closure can't.
    print(apply(|x| max(x, 3), 1)); // expect: 3
    print(apply(|x| x + 1, 1)); // expect: 2

    let f: fn(i64, i64) -> i64 with panic = max;
    print(f(4, 5)); // expect: 5
    let g: fn(i64) -> i64 with panic = inc;
    print(g(4)); // expect: 5
}

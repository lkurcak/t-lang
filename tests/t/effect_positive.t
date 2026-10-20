// What effects allow: a function that can't panic, a closure's effects
// passed on through an effect parameter, and a function type gaining an
// effect.
fn max(a: int, b: int) -> int { if a < b { b } else { a } }

fn apply<e, F: Fn(int) -> int with e + Discard>(f: F, x: int) -> int with e { f(x) }

fn inc(n: int) -> int with panic { n + 1 }

fn main() with panic {
    // `apply` can't panic when its closure can't.
    print(apply(|x| max(x, 3), 1)); // expect: 3
    print(apply(|x| x + 1, 1)); // expect: 2

    let f: fn(int, int) -> int with panic = max;
    print(f(4, 5)); // expect: 5
    let g: fn(int) -> int with panic = inc;
    print(g(4)); // expect: 5
}

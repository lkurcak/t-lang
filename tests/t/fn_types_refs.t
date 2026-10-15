// A function type with references in it takes references of any lifetime:
// `fn(&int) -> int` is a function that works for every lifetime, like a
// generic function, and each call picks one.

fn apply(f: fn(&int) -> int, x: &int) -> int { f(x) }

fn twice(f: fn(&int) -> &int, x: &int) -> int { *f(f(x)) }

fn same(r: &int) -> &int { r }

fn main() {
    let n = 3;
    print(apply(|r| *r + 1, &n));   // expect: 4
    print(twice(same, &n));         // expect: 3
    print(twice(|r| r, &n));        // expect: 3
    let g: fn(&int) -> int = |r| *r * 10;
    let a = 1;
    let b = 2;
    print(g(&a) + g(&b));           // expect: 30
}

// Range patterns: `a..b` excludes `b`, `a..=b` includes it, and either end
// can be left open.

fn bucket(n: int) -> int {
    match n {
        ..0 => -1,
        0 => 0,
        1..10 => 1,
        10..=99 => 2,
        100.. => 3,
    }
}

// Inclusive at the top, open at the bottom, and negative bounds.
fn sign(n: int) -> int {
    match n {
        ..=-1 => -1,
        0 => 0,
        1.. => 1,
    }
}

// A later range may contain an earlier one, as a fallback around a special
// case. (If the earlier one covered an end of it, it should be narrowed: see
// `warn_range_trim_*.t`.)
fn overlap(n: int) -> int {
    match n {
        5..10 => 1,
        0..20 => 2,
        _ => 3,
    }
}

enum Opt { None, Some(int) }

// Nested, in or-patterns, and covering every int between them.
fn small(o: Opt) -> bool {
    match o {
        Opt::Some(0..3 | 7) => true,
        Opt::Some(..0 | 3..) | Opt::None => false,
    }
}

struct P { x: int, y: int }

// `..` still ignores the rest of a struct's fields.
fn quadrant(p: P) -> int {
    match p {
        P { x: 0.., y: 0.. } => 1,
        P { x: ..0, .. } => 2,
        P { .. } => 4,
    }
}

// The `x` ranges overlap, but `Q { x: 10, on: false }` still reaches the
// second arm, so `10..30` is as narrow as it can be.
struct Q { x: int, on: bool }

fn lit(q: Q) -> int {
    match q {
        Q { x: 0..20, on: true } => 1,
        Q { x: 10..30, .. } => 2,
        _ => 3,
    }
}

// The `on: true` combination reaches `x` from 10, the `on: false` one from 5.
// Together they reach all of `5..20`.
fn either(q: Q) -> int {
    match q {
        Q { x: 0..10, on: true } => 1,
        Q { x: 5..20, on: true | false } => 2,
        _ => 3,
    }
}

fn main() {
    print(bucket(-5));      // expect: -1
    print(bucket(0));       // expect: 0
    print(bucket(1));       // expect: 1
    print(bucket(9));       // expect: 1
    print(bucket(10));      // expect: 2
    print(bucket(99));      // expect: 2
    print(bucket(100));     // expect: 3
    print(sign(-9223372036854775807));  // expect: -1
    print(sign(0));         // expect: 0
    print(sign(9223372036854775807));   // expect: 1
    print(overlap(7));      // expect: 1
    print(overlap(2));      // expect: 2
    print(overlap(12));     // expect: 2
    print(overlap(25));     // expect: 3
    print(lit(Q { x: 15, on: true }));      // expect: 1
    print(lit(Q { x: 15, on: false }));     // expect: 2
    print(lit(Q { x: 25, on: true }));      // expect: 2
    print(either(Q { x: 5, on: true }));    // expect: 1
    print(either(Q { x: 5, on: false }));   // expect: 2
    print_bool(small(Opt::Some(2)));    // expect: true
    print_bool(small(Opt::Some(3)));    // expect: false
    print_bool(small(Opt::Some(7)));    // expect: true
    print_bool(small(Opt::Some(-1)));   // expect: false
    print_bool(small(Opt::None));       // expect: false
    print(quadrant(P { x: 1, y: 2 }));  // expect: 1
    print(quadrant(P { x: -1, y: 2 })); // expect: 2
    print(quadrant(P { x: 1, y: -2 })); // expect: 4
    // Irrefutable, so fine in `let`.
    let Opt::Some(..0) | Opt::Some(0..) | Opt::None = Opt::Some(5);
    print(1);               // expect: 1
}

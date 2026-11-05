// A closure that captures nothing can be used where a function type is
// wanted: it's then its body, a plain function. Wherever a `fn` is wanted,
// in an annotation, an argument, a result or a field, and through `if`,
// `match` and array literals to their parts.
fn apply(f: fn(i64) -> i64 with panic, x: i64) -> i64 with panic { f(x) }
fn inc(x: i64) -> i64 with panic { x + 1 }
fn pick(double: bool) -> fn(i64) -> i64 with panic {
    if double { |x| x * 2 } else { inc }
}
fn choose(n: u8) -> fn(i64) -> i64 with panic {
    match n {
        0 => |x| x,
        1 => inc,
        _ => |x| x * x,
    }
}
fn generic<F: Fn(i64) -> i64 with panic + Discard>(f: F) -> i64 with panic { f(7) }

#[derive(Copy, Discard)]
struct Op { f: fn(i64) -> i64 with panic }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let neg = |x: i64| 0 - x;
    let f: fn(i64) -> i64 with panic = neg;
    out.line(&f(3))?;                        // expect: -3
    out.line(&apply(neg, 4))?;               // expect: -4
    out.line(&apply(|x| x + 10, 4))?;        // expect: 14
    out.line(&pick(true)(5))?;               // expect: 10
    out.line(&pick(false)(5))?;              // expect: 6
    out.line(&(choose(0)(3) + choose(1)(3) + choose(2)(3)))?; // expect: 16
    let fs: [fn(i64) -> i64 with panic; 3] = [|x| x + 1, inc, neg];
    out.line(&(fs[0](1) + fs[1](1) + fs[2](1)))?; // expect: 3
    let op = Op { f: |x| x * 5 };
    out.line(&(op.f)(2))?;                   // expect: 10
    let minus: fn(i64) -> i64 with panic = |x| x - 2;
    let o = Some(minus);
    match o {
        Some(g) => out.line(&g(2))?, // expect: 0
        None => {}
    }
    // A function made from a closure is a function: generic code takes it
    // as one.
    out.line(&generic(f))?;                  // expect: -7
    // A variable of a function type takes closures of different types.
    let mut g: fn(i64) -> i64 with panic = |x| x;
    g = |x| x + 1;
    out.line(&g(1))?;                        // expect: 2
    // One that can't panic is a function that may.
    let id = |x: i64| x;
    let h: fn(i64) -> i64 with panic = id;
    out.line(&h(1))?;                        // expect: 1
    Ok(())
}

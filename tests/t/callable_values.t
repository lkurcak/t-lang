// A closure's value carries what every run of it gives, and a function's
// value is that function: wherever the value goes, given for a callee's
// `F` or called, what it gives back is known (docs/78).
use alloc::list::List;

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn app<T, R, F: FnOnce(T) -> R + Discard>(x: T, f: F) -> R { f(x) }

fn five() -> { v: u32 | v > 0 } { 5 }

fn half(x: u32) -> { r: u32 | r <= x } { x / 2 }

fn below(n: usize) -> { v: usize | v <= n } { n / 2 }

fn id<T>(x: T) -> T { x }

fn push1(l: &mut List<u8>) with alloc ensures l.len() == old(l.len()) + 1 {
    l.push(1);
}

#[allow(dead_code)]
struct Five {}

impl Five {
    fn make() -> { v: u32 | v > 0 } { 5 }
}

#[derive(Discard)]
struct Holder<F> { f: F }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    // A function given for `F`, and a method.
    let a: { v: u32 | v > 0 } = call(five);
    out.line(&a)?; // expect: 5
    let b: { v: u32 | v > 0 } = call(Five::make);
    out.line(&b)?; // expect: 5
    // A closure made before the call, and moved first.
    let f = || five();
    let c: { v: u32 | v > 0 } = call(f);
    out.line(&c)?; // expect: 5
    let g0 = || five();
    let g = g0;
    let d: { v: u32 | v > 0 } = call(g);
    out.line(&d)?; // expect: 5
    // A function's value in a variable.
    let p = five;
    let e: { v: u32 | v > 0 } = call(p);
    out.line(&e)?; // expect: 5
    // Called straight: a closure's value, one made in place, a function's.
    let f2 = || five();
    let h: { v: u32 | v > 0 } = f2();
    out.line(&h)?; // expect: 5
    let i: { v: u32 | v > 0 } = (|| five())();
    out.line(&i)?; // expect: 5
    let j: { v: u32 | v > 0 } = p();
    out.line(&j)?; // expect: 5
    // A closure another one captures and calls.
    let f3 = || five();
    let k: { v: u32 | v > 0 } = call(|| f3());
    out.line(&k)?; // expect: 5
    // A function a closure gives back is that function.
    let mk = || five;
    let five_again = mk();
    let z: { v: u32 | v > 0 } = five_again();
    out.line(&z)?; // expect: 5
    // A closure in a field.
    let hold = Holder { f: || five() };
    let m: { v: u32 | v > 0 } = call(hold.f);
    out.line(&m)?; // expect: 5
    // The same closure after an `if`.
    let f4 = || five();
    if a > 3 {
        out.line(&"big")?; // expect: big
    }
    let n: { v: u32 | v > 0 } = call(f4);
    out.line(&n)?; // expect: 5
    // A function given for `F` is as the closure calling it would be: its
    // parameter is what the call gives it.
    let x: { v: u32 | v < 10 } = 9;
    let q: { v: u32 | v < 10 } = app(x, half);
    out.line(&q)?; // expect: 4
    let r: { v: u32 | v < 10 } = app(x, |v| half(v));
    out.line(&r)?; // expect: 4
    let s: { v: u32 | v < 10 } = app(x, id);
    out.line(&s)?; // expect: 9
    // A closure's value called with arguments: its parameters are them.
    let pick = |t: bool| if t { 1u32 } else { 2u32 };
    let t: { v: u32 | v == 1 } = pick(true);
    out.line(&t)?; // expect: 1
    let half_of = |w: u32| w / 2;
    let u: { v: u32 | v <= 6 } = half_of(12);
    out.line(&u)?; // expect: 6
    // A function's value called: what its signature says.
    let bl = below;
    let w: { v: usize | v <= 4 } = bl(4);
    out.line(&w)?; // expect: 2
    let mut l: List<u8> = List::new();
    let pu = push1;
    pu(&mut l);
    let one: { v: usize | v == 1 } = l.len();
    out.line(&one)?; // expect: 1
    Ok(())
}

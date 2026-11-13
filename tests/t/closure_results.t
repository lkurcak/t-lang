// What a closure gives back, given straight to a call, is what its body
// proves of it, at each place it returns: so a generic function that gives
// back what the closure did gives back that (docs/74).
use core::sync::Mutex;

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn five() -> { v: u32 | v > 0 } { 5 }

fn twice<R, F: FnMut() -> R + Discard>(mut f: F) -> (R, R) { (f(), f()) }

fn app<T, R, F: FnOnce(T) -> R + Discard>(x: T, f: F) -> R { f(x) }

fn pick<R: Discard, F: FnOnce() -> R + Discard>(r: R, c: bool, f: F) -> R {
    match c {
        true => f(),
        false => r,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a: { v: u32 | v > 0 } = call(|| five());
    out.line(&a)?; // expect: 5
    // In an enum, and through `unwrap`.
    let o: Option<{ v: u32 | v > 0 }> = call(|| Some(five()));
    out.line(&o.unwrap())?; // expect: 5
    // A closure that runs twice: each result has it.
    let (p, q) = twice(|| five());
    let p: { v: u32 | v > 0 } = p;
    let q: { v: u32 | v > 0 } = q;
    out.line(&p)?; // expect: 5
    out.line(&q)?; // expect: 5
    // What the closure's parameter is: what the call's argument was.
    let x: { v: u32 | v < 10 } = 4;
    let y: { v: u32 | v <= 10 } = app(x, |z| z + 1);
    out.line(&y)?; // expect: 5
    // Several returns, and an argument of the same type.
    let r: { v: u32 | v >= 3 } = pick(3, true, || {
        if a > 7 {
            return 9;
        }
        4
    });
    out.line(&r)?; // expect: 4
    // A captured value.
    let k: { v: u32 | v < 100 } = 40;
    let s: { v: u32 | v < 101 } = call(move || k + 1);
    out.line(&s)?; // expect: 41
    // `unwrap_or_else`, `Option::map`.
    let none: Option<{ v: u32 | v > 0 }> = None;
    let w: { v: u32 | v > 0 } = none.unwrap_or_else(|| five());
    out.line(&w)?; // expect: 5
    let m: Option<{ v: u32 | v > 0 }> = Some(2u32).map(|_| five());
    out.line(&m.unwrap())?; // expect: 5
    // `Mutex::lock`.
    let mx: Mutex<u32> = Mutex::new(12);
    let small: { v: u32 | v <= 10 } = mx.lock(|n| {
        if *n > 10 {
            return 10;
        }
        *n
    });
    out.line(&small)?; // expect: 10
    // A thread's result, through its handle.
    let h = os.threads.spawn(|| five());
    let t: { v: u32 | v > 0 } = h.join();
    out.line(&t)?; // expect: 5
    Ok(())
}

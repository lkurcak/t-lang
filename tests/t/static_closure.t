// A closure that uses a static has the effect `global`, which a bound must
// allow; one that's handed a reference to it doesn't.
use core::sync::Atomic;

static TOTAL: Atomic<i64> = Atomic::new(0);

fn each<effect e, F: Fn(i64) with e + Discard>(f: F) with e {
    f(1);
    f(2);
}

fn add_to(t: &Atomic<i64>, n: i64) {
    let _ = t.fetch_add(n);
}

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    each(|n| {
        let _ = TOTAL.fetch_add(n);
    });
    let t = &TOTAL;
    each(|n| add_to(t, n));
    out.line(&TOTAL.load())?; // expect: 6
    Ok(())
}

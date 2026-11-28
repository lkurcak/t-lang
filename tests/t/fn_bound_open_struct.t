// A type's `Fn` bound with no `with` says nothing of what its calls do: a
// `Holder` holds a closure that may panic, and what calling it does is up
// to the function that calls it, by its own bound.
#[derive(Discard where F: Discard)]
struct Holder<F: Fn() -> i64> { f: F }

fn call<F: Fn() -> i64>(h: &Holder<F>) -> i64 { (h.f)() }

fn quiet<F: Fn() -> i64 with {}>(h: &Holder<F>) -> i64 with {} { (h.f)() }

fn five() -> i64 { 5 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    // `n + 1` may overflow, for all the checker knows of `n`.
    let n = five();
    let h = Holder { f: move || n + 1 };
    out.line(&call(&h))?; // expect: 6
    let q = Holder { f: || 7 };
    out.line(&quiet(&q))?; // expect: 7
    Ok(())
}

// Closures can be stored, as long as what they borrow outlives them.
#[derive(Discard)]
struct Counter<F> { next: F, calls: i64 }

fn step<effect e, F: FnMut() -> i64 with e>(c: &mut Counter<F>) -> i64 with {panic, e} {
    c.calls = c.calls + 1;
    (c.next)()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut i = 0;
    let mut c = Counter { next: || { i = i + 10; i }, calls: 0 };
    out.line(&step(&mut c))?;            // expect: 10
    out.line(&step(&mut c))?;            // expect: 20
    out.line(&c.calls)?;                 // expect: 2
    out.line(&i)?;                       // expect: 20
    Result::Ok(())
}

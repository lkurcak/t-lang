// A `move` closure takes a `&mut` by value, a new borrow of what it points
// to, and writes through it: what was known of `r` before is forgotten.
#[derive(Copy, Discard)]
struct C { n: i64 }

impl C {
    #[measure]
    fn get(&self) -> i64 { self.n }
}

fn f(r: &mut C) ensures r.get() == 1 {
    r.n = 1;
    let mut c = move || {
        r.n = 0;
    };
    c();
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x = C { n: 5 };
    f(&mut x);
    out.line(&x.n)?;
    Ok(())
}
// error: can't prove `r.get() == 1` for what `f` ensures

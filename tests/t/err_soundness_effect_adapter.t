// An impl's associated effect must cover what its methods do: here `next`
// calls `f`, whose effects are `f`, which `e` leaves out.
#[derive(Discard where I: Discard, F: Discard)]
struct Apply<I, F> { iter: I, f: F }

impl<effect f, I: Iterator<Item = i64>, F: FnMut(i64) -> i64 with f> Iterator for Apply<I, F> {
    type Item = i64;
    effect e = I::e;

    fn next(&mut self) -> Option<i64> with Self::e {
        match self.iter.next() {
            Some(x) => Some((self.f)(x)),
            None => None,
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut a = Apply { iter: 0..3i64, f: |x: i64| x / 0 };
    out.line(&a.next().unwrap())?;
    Ok(())
}
// error: this call may have the effects `f`, but `next` doesn't say `with f`

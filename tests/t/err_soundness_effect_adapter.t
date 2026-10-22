// An impl's associated effect must cover what its methods do: here `next`
// calls `f`, whose effects are `f`, which `e` leaves out.
#[derive(Discard where I: Discard, F: Discard)]
struct Apply<I, F> { iter: I, f: F }

impl<f, I: Iterator<Item = i64>, F: FnMut(i64) -> i64 with f> Iterator for Apply<I, F> {
    type Item = i64;
    effect e = I::e;

    fn next(&mut self) -> Option<i64> with Self::e {
        match self.iter.next() {
            Option::Some(x) => Option::Some((self.f)(x)),
            Option::None => Option::None,
        }
    }
}

fn main() with panic {
    let mut a = Apply { iter: 0..3i64, f: |x: i64| x / 0 };
    print(a.next().unwrap());
}
// error: this call may have the effects `f`, but `next` doesn't say `with f`

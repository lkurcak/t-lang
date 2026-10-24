// A supertrait: implementing `Ord` needs `Eq` implemented too, and a bound
// `T: Ord` gives `T` the methods of both, and `==`.

trait Ord: Eq + Discard {
    fn lt(&self, other: &Self) -> bool;

    fn max(self, other: Self) -> Self {
        if self.lt(&other) { other } else { self }
    }
}

#[derive(Discard, Copy, Eq)]
struct Money { cents: i64 }

impl Ord for Money {
    fn lt(&self, other: &Money) -> bool { self.cents < other.cents }
}

impl Ord for i64 {
    fn lt(&self, other: &i64) -> bool { *self < *other }
}

fn largest<T: Ord + Copy>(a: T, b: T, c: T) -> T { a.max(b).max(c) }

fn same<T: Ord>(a: &T, b: &T) -> bool { a == b && !a.lt(b) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let m = largest(Money { cents: 3 }, Money { cents: 9 }, Money { cents: 5 });
    out.line(&m.cents)?;                            // expect: 9
    out.line(&largest(4i64, 1, 2))?;                   // expect: 4
    out.line(&same(&m, &Money { cents: 9 }))?; // expect: true
    out.line(&same(&1i64, &2))?;                  // expect: false
    Result::Ok(())
}

// A supertrait: implementing `Ord` needs `Eq` implemented too, and a bound
// `T: Ord` gives `T` the methods of both, and `==`.

trait Ord: Eq {
    fn lt(&self, other: &Self) -> bool;

    fn max(self, other: Self) -> Self {
        if self.lt(&other) { other } else { self }
    }
}

#[derive(Copy, Eq)]
struct Money { cents: int }

impl Ord for Money {
    fn lt(&self, other: &Money) -> bool { self.cents < other.cents }
}

impl Ord for int {
    fn lt(&self, other: &int) -> bool { *self < *other }
}

fn largest<T: Ord + Copy>(a: T, b: T, c: T) -> T { a.max(b).max(c) }

fn same<T: Ord>(a: &T, b: &T) -> bool { a == b && !a.lt(b) }

fn main() {
    let m = largest(Money { cents: 3 }, Money { cents: 9 }, Money { cents: 5 });
    print(m.cents);                            // expect: 9
    print(largest(4, 1, 2));                   // expect: 4
    print_bool(same(&m, &Money { cents: 9 })); // expect: true
    print_bool(same(&1, &2));                  // expect: false
}

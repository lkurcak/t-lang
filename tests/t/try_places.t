// `?` in a method, in a loop (where it returns from the function, not the
// loop), nested in another `?`'s operand, and in a nested function.
#[derive(Discard, Copy)]
struct Bag { n: i64 }

impl Bag {
    fn take(self, k: i64) -> Option<Bag> with panic {
        if k <= self.n { Option::Some(Bag { n: self.n - k }) } else { Option::None }
    }

    fn take_all(self, ks: core::ops::Range<i64>) -> Option<Bag> with panic {
        let mut bag = self;
        for k in ks {
            bag = bag.take(k)?;
        }
        Option::Some(bag)
    }
}

fn wrap(n: i64) -> Option<Option<i64>> { Option::Some(if n > 0 { Option::Some(n) } else { Option::None }) }

fn main() with panic {
    fn twice(n: i64) -> Option<i64> with panic { Option::Some(wrap(n)?? * 2) }
    let bag = Bag { n: 10 };
    print(bag.take_all(1..4).unwrap_or(Bag { n: -1 }).n); // expect: 4
    print(bag.take_all(3..6).unwrap_or(Bag { n: -1 }).n); // expect: -1
    print(twice(4).unwrap_or(0)); // expect: 8
    print(twice(-4).unwrap_or(0)); // expect: 0
}

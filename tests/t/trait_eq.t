// `==` is `Eq`'s: `#[derive(Eq)]` compares field by field, and an impl of
// its own can say anything, which a derived `Eq` of a type around it uses.

#[derive(Discard, Eq)]
enum Option<T> { Some(T), None }

#[derive(Discard)]
struct Money { cents: int }

// Equal to the dollar.
impl Eq for Money {
    fn eq(&self, other: &Money) -> bool { self.cents / 100 == other.cents / 100 }
}

#[derive(Discard, Eq)]
struct Account { id: int, balance: Money }

fn main() {
    print_bool(Money { cents: 101 } == Money { cents: 199 });   // expect: true
    print_bool(Money { cents: 101 } != Money { cents: 201 });   // expect: true
    let a = Account { id: 1, balance: Money { cents: 150 } };
    let b = Account { id: 1, balance: Money { cents: 120 } };
    print_bool(a == b);                                         // expect: true
    print_bool(Option::Some(Money { cents: 5 }) == Option::Some(Money { cents: 95 })); // expect: true
    print_bool(Option::Some(1) == Option::None);                // expect: false
    // `eq` is a method like any other.
    print_bool(1.eq(&1));                                       // expect: true
    print_bool(Eq::eq(&a, &b));                                 // expect: true
    print_bool(a.balance.eq(&Money { cents: 300 }));            // expect: false
}

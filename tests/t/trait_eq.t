// `==` is `Eq`'s: `#[derive(Eq)]` compares field by field, and an impl of
// its own can say anything, which a derived `Eq` of a type around it uses.

#[derive(Discard, Eq)]
enum Option<T> { Some(T), None }

#[derive(Discard)]
struct Money { cents: i64 }

// Equal to the dollar.
impl Eq for Money {
    fn eq(&self, other: &Money) -> bool { self.cents / 100 == other.cents / 100 }
}

#[derive(Discard, Eq)]
struct Account { id: i64, balance: Money }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(Money { cents: 101 } == Money { cents: 199 }))?;   // expect: true
    out.line(&(Money { cents: 101 } != Money { cents: 201 }))?;   // expect: true
    let a = Account { id: 1, balance: Money { cents: 150 } };
    let b = Account { id: 1, balance: Money { cents: 120 } };
    out.line(&(a == b))?;                                         // expect: true
    out.line(&(Option::Some(Money { cents: 5 }) == Option::Some(Money { cents: 95 })))?; // expect: true
    out.line(&(Option::Some(1i64) == Option::None))?;                // expect: false
    // `eq` is a method like any other.
    out.line(&1i64.eq(&1i64))?;                                       // expect: true
    out.line(&Eq::eq(&a, &b))?;                                 // expect: true
    out.line(&a.balance.eq(&Money { cents: 300 }))?;            // expect: false
    Result::Ok(())
}

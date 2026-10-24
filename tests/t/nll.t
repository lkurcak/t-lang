// A borrow lasts only as long as the reference is used, not to the end of
// its block: *non-lexical lifetimes*.

#[derive(Discard)]
struct Account { balance: i64 }

fn deposit(a: &mut Account, n: i64) with panic { a.balance += n; }

fn balance(a: &Account) -> i64 { a.balance }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut x: i64 = 1;
    let r = &x;
    out.line(&(*r))?;                  // expect: 1
    x = 2;                      // `r` isn't used after this, so it's fine
    out.line(&x)?;                   // expect: 2

    let mut acct = Account { balance: 10 };
    let m = &mut acct;
    deposit(m, 5);
    deposit(m, 5);              // `m` can be used again: each call reborrows it
    out.line(&balance(&acct))?;      // expect: 20

    // Two `&mut`s to different fields at once.
    let mut pair = Account { balance: 0 };
    let mut other = Account { balance: 0 };
    let a = &mut pair.balance;
    let b = &mut other.balance;
    *a += 1;
    *b += 2;
    out.line(&(pair.balance + other.balance))?; // expect: 3

    // A borrow in a loop, a new one each time round.
    let mut total: i64 = 0;
    let mut i = 0;
    while i < 4 {
        let t = &mut total;
        *t += i;
        i += 1;
    }
    out.line(&total)?;               // expect: 6

    // In one branch only.
    let mut y: i64 = 5;
    let s = &y;
    if *s > 3 {
        y = 0;
    } else {
        out.line(&(*s))?;
    }
    out.line(&y)?;                   // expect: 0
    Result::Ok(())
}

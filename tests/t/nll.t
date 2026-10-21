// A borrow lasts only as long as the reference is used, not to the end of
// its block: *non-lexical lifetimes*.

#[derive(Discard)]
struct Account { balance: int }

fn deposit(a: &mut Account, n: int) with panic { a.balance += n; }

fn balance(a: &Account) -> int { a.balance }

fn main() with panic {
    let mut x = 1;
    let r = &x;
    print(*r);                  // expect: 1
    x = 2;                      // `r` isn't used after this, so it's fine
    print(x);                   // expect: 2

    let mut acct = Account { balance: 10 };
    let m = &mut acct;
    deposit(m, 5);
    deposit(m, 5);              // `m` can be used again: each call reborrows it
    print(balance(&acct));      // expect: 20

    // Two `&mut`s to different fields at once.
    let mut pair = Account { balance: 0 };
    let mut other = Account { balance: 0 };
    let a = &mut pair.balance;
    let b = &mut other.balance;
    *a += 1;
    *b += 2;
    print(pair.balance + other.balance); // expect: 3

    // A borrow in a loop, a new one each time round.
    let mut total = 0;
    let mut i = 0;
    while i < 4 {
        let t = &mut total;
        *t += i;
        i += 1;
    }
    print(total);               // expect: 6

    // In one branch only.
    let mut y = 5;
    let s = &y;
    if *s > 3 {
        y = 0;
    } else {
        print(*s);
    }
    print(y);                   // expect: 0
}

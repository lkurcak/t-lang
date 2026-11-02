// A cell of a value that can't be thrown away: `replace` gives the old one
// back to use up, and the last is taken out with `into_inner`.
use core::cell::Cell;

struct Ticket { n: i64 }

fn spend(t: Ticket) -> i64 { let Ticket { n } = t; n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let c = Cell::new(Ticket { n: 1 });
    let r = &c;
    let first = spend(r.replace(Ticket { n: 2 }));
    let second = spend(c.into_inner());
    out.line(&first)?; // expect: 1
    out.line(&second)?; // expect: 2
    Result::Ok(())
}

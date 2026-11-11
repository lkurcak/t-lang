// A refinement alias names a refined type: each use means its refinement
// written out there, as a parameter's, a result's, a `let`'s, a field's,
// or a type argument's at any depth.
use alloc::list::List;

type Square = { s: u8 | s < 64 };
type Squares = List<Square>;
type Row = [Square; 2];

#[derive(Copy, Discard)]
struct Move { from: Square, to: Square }

fn step(s: Square, d: { d: u8 | d < 8 }) -> Option<Square> {
    match s + d < 64 {
        true => Some(s + d),
        false => None,
    }
}

fn need(_s: { s: u8 | s < 64 }) {}

// A refinement can be written on top of an alias's: both hold.
fn nonzero(s: { s: Square | s != 0 }) -> Square { s - 1 }

fn first(l: &Squares) -> Square {
    match l.get(0) {
        Some(s) => *s,
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: Square = 60;
    need(a);
    let m = Move { from: a, to: 3 };
    need(m.from);
    need(m.to);
    match step(m.to, 7) {
        Some(b) => {
            need(b);
            out.line(&b)?; // expect: 10
        },
        None => {},
    }
    out.line(&nonzero(m.to))?; // expect: 2
    let mut l: Squares = List::new();
    l.push(m.from);
    out.line(&first(&l))?; // expect: 60
    let r: Row = [a, 1];
    need(r[1]);
    let t: (Square, &Square) = (r[0], &a);
    need(t.0);
    need(*t.1);
    Ok(())
}

// A variant's field can have a refinement, as a struct's can: every value
// of the variant has it. Making one proves it, and a `match` learns it. A
// field's refinement can name the fields before it, by their binders, as a
// parameter's names the parameters before it.
use alloc::list::List;

type Square = { s: u8 | s < 64 };

#[derive(Copy, Discard)]
enum Key {
    Digit({ d: u8 | d < 10 }),
    Range({ lo: u8 | lo < 100 }, { hi: u8 | hi >= lo }),
    Go(Square, Square),
    Other,
}

#[derive(Discard)]
enum Bag {
    Digits(List<{ d: u8 | d < 10 }>),
    Empty,
}

#[derive(Copy, Discard)]
enum At<const N: usize> {
    Index({ i: usize | i < N }),
}

fn need_digit(_d: { d: u8 | d < 10 }) {}
fn need_square(_s: { s: u8 | s < 64 }) {}

fn width(k: Key) -> u8 {
    match k {
        Key::Digit(d) => {
            need_digit(d);
            d
        },
        // Proved: `hi >= lo`.
        Key::Range(lo, hi) => hi - lo,
        Key::Go(a, b) => {
            need_square(a);
            need_square(b);
            0
        },
        Key::Other => 0,
    }
}

fn first(b: &Bag) -> u8 {
    match b {
        Bag::Digits(l) => match l.get(0) {
            Some(d) => {
                need_digit(*d);
                *d
            },
            None => 0,
        },
        Bag::Empty => 0,
    }
}

fn get<const N: usize>(a: &[u8; N], at: At<N>) -> u8 {
    match at {
        At::Index(i) => a[i],
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&width(Key::Digit(7)))?; // expect: 7
    out.line(&width(Key::Range(3, 10)))?; // expect: 7
    out.line(&width(Key::Go(63, 0)))?; // expect: 0
    out.line(&width(Key::Other))?; // expect: 0
    let mut l: List<{ d: u8 | d < 10 }> = List::new();
    l.push(4);
    out.line(&first(&Bag::Digits(l)))?; // expect: 4
    out.line(&first(&Bag::Empty))?; // expect: 0
    out.line(&get(&[1u8, 2, 3], At::Index(2)))?; // expect: 3
    Ok(())
}

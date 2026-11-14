// `extend_from_slice` puts copies of a slice's values at a list's end,
// and says how long the list is then; `from_slice` makes a new list of
// them.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;

    let mut l: List<u32> = List::new();
    l.push(1);
    let more = [2u32, 3, 4, 5, 6, 7, 8, 9, 10];
    l.extend_from_slice(more.as_slice());
    // The length is known: 1 + 9.
    let last = l[9];
    out.line(&f"{l.len()} {last}")?; // expect: 10 10

    // Again, from another list, past the capacity it has.
    let other = List::filled(25, 0u32);
    l.extend_from_slice(other.as_slice());
    out.line(&f"{l.len()} {l[34]}")?; // expect: 35 0

    // A copy of a list's values.
    let copy = List::from_slice(l.as_slice());
    let first = copy[0];
    let n = copy.len();
    out.line(&f"{n} {first} {copy == l}")?; // expect: 35 1 true

    // Of nothing.
    let empty: List<u8> = List::from_slice(core::slice::empty());
    out.line(&empty.len())?; // expect: 0

    // Refined values keep their refinement.
    let mut digits: List<{ v: u8 | v < 10 }> = List::new();
    let some: [{ v: u8 | v < 10 }; 2] = [4, 5];
    digits.extend_from_slice(some.as_slice());
    let sum: u8 = digits[0] + digits[1];
    out.line(&sum)?; // expect: 9
    Ok(())
}

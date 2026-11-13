// What's lent of a field is closed at what the fields it names are now:
// one of those lent too, and changed, makes the field's refinement false,
// which the struct's being proved again after the call finds.
use alloc::list::List;

fn put<T, U: Discard>(_a: &mut T, b: &mut U, x: U) { *b = x; }

#[derive(Discard)]
struct Band {
    rows: usize,
    now: { l: List<u8> | l.len() == rows },
}

fn main() with alloc {
    let mut band = Band { rows: 2, now: List::filled(2, 0) };
    put(&mut band.now, &mut band.rows, 5);
}
// error: can't prove `l.len() == rows` for field `now` of `Band`

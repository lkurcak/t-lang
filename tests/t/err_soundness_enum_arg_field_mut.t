// A field whose type has a refined type argument can't be written through
// a `&mut` either, as match ergonomics would make one.
struct Cursor {
    len: usize,
    at: Option<{ i: usize | i < len }>,
}

fn bump(c: &mut Cursor) {
    match &mut c.at {
        Some(i) => *i = *i + 1,
        None => {}
    }
}

fn main() {}
// error: can't borrow `c.at` as `&mut`: field `at` of `Cursor` has a refinement

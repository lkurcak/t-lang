// Two `get_mut`s of one slice can't both live: they might be the same
// value. `split_at_mut` is how to change two at once.
fn both(s: &mut [int]) with panic {
    let a = s.get_mut(0).unwrap();
    let b = s.get_mut(1).unwrap();
    *a = *b;
}

fn main() {}
// error: cannot borrow `*s` as mutable more than once at a time

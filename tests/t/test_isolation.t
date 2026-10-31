// Each test is a program of its own: one that panics while it holds memory
// leaves nothing behind for the next, which starts from nothing, and the
// check that everything was freed is each's own.
use alloc::boxed::Box;
use alloc::list::List;

fn main() {}

#[test]
fn panics_holding_memory() {
    let mut xs: List<u8> = List::new();
    xs.push(1);
    let b = Box::new(2u8);
    assert_eq(&*b, &3);
    let _ = *b;
}

#[test]
fn frees_its_own() {
    let mut xs: List<u8> = List::new();
    xs.push(1);
    assert_eq(&xs.len(), &1);
}

// test: panics_holding_memory ... FAILED: left: 2 right: 3
// test: frees_its_own ... ok

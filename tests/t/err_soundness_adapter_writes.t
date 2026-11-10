// An iterator adapter over `iter_mut` gives what its iterator gives: a
// `&mut` to each element, whose refinement each write through it must keep.
// (Before step 63 an adapter's items, of type `I::Item`, had nothing
// declared behind them.)
#![allow(dead_code)]
use alloc::list::List;

fn skipped(l: &mut List<{ t: usize | t < 5 }>) {
    for x in l.iter_mut().skip(0) {
        *x = 99;
    }
}

fn reversed(l: &mut List<{ t: usize | t < 5 }>) {
    for x in l.iter_mut().rev() {
        *x = 99;
    }
}

fn enumerated(l: &mut List<{ t: usize | t < 5 }>) with panic {
    for (_, x) in l.iter_mut().enumerate() {
        *x = 99;
    }
}

fn zipped(l: &mut List<{ t: usize | t < 5 }>) {
    for (x, _) in l.iter_mut().zip(0..1u8) {
        *x = 99;
    }
}

fn mapped(l: &mut List<{ t: usize | t < 5 }>) {
    for x in l.iter_mut().map(|r| r) {
        *x = 99;
    }
}

fn main() {}
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`

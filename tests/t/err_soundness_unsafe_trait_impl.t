// An impl's method can do no more than its trait's says: callers through
// the trait see no `unsafe`.
use alloc::raw::RawBuf;

trait Take {
    fn take(&mut self) -> int;
}

impl Take for RawBuf<int> {
    fn take(&mut self) -> int with unsafe { self.read(0) }
}

fn main() {}
// error: `take` is unsafe, but `Take`'s `take` can't, so its impls can't either

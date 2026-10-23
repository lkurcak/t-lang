// `Try`'s methods can't panic, so neither can a `?`.
use core::ops::{ControlFlow, Try};

#[derive(Discard)]
struct W { v: i64 }

impl Try for W {
    type Output = i64;
    type Residual = W;

    fn branch(self) -> ControlFlow<W, i64> with panic {
        if self.v + 1 > 0 { ControlFlow::Continue(self.v) } else { ControlFlow::Break(self) }
    }

    fn from_residual(r: W) -> W { r }
}

fn main() {}
// error: `branch` may panic, but `Try`'s `branch` can't, so its impls can't either

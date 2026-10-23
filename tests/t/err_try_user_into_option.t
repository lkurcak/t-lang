// A program's own `Try` type has its own residual, which an `Option` isn't
// made from.
use core::ops::{ControlFlow, Try};

#[derive(Discard)]
struct Code { code: i64 }

impl Try for Code {
    type Output = ();
    type Residual = Code;

    fn branch(self) -> ControlFlow<Code, ()> {
        if self.code == 0 { ControlFlow::Continue(()) } else { ControlFlow::Break(self) }
    }

    fn from_residual(r: Code) -> Code { r }
}

fn f() -> Option<i64> {
    Code { code: 1 }?;
    Option::Some(1)
}

fn main() { let _ = f(); }
// error: `?` can't return early here: `f` returns `Option<i64>`, so a `?` in it must stop with `Option<!>`, and this one stops with `Code`

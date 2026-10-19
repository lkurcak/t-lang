// `f` never returns, so there's no type behind its `impl Show`.
trait Show {
    fn show(&self) -> int;
}

impl Show for int {
    fn show(&self) -> int { *self }
}

fn f() -> impl Show { loop {} }

fn main() { print(f().show()); }
// error: can't tell what type `f` returns

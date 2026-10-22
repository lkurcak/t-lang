// `f` never returns, so there's no type behind its `impl Show`.
trait Show {
    fn show(&self) -> i64;
}

impl Show for i64 {
    fn show(&self) -> i64 { *self }
}

fn f() -> impl Show { loop {} }

fn main() { print(f().show()); }
// error: can't tell what type `f` returns

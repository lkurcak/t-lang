trait Show {
    fn show(&self) -> int;
}

fn f() -> impl Show { 1 }

fn main() { print(f().show()); }
// error: `int` doesn't implement `Show`, which `f`'s return type says

trait Show {
    fn show(&self) -> i64;
}

fn f() -> impl Show { 1i64 }

fn main() { print(f().show()); }
// error: `i64` doesn't implement `Show`, which `f`'s return type says

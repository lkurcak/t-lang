// A module sees only its own items and what it imports, not those of the
// module around it.
fn helper() -> i64 { 1 }

mod m {
    pub fn f() -> i64 { helper() }
}

fn main() { print(m::f()); }
// error: unknown variable `helper`

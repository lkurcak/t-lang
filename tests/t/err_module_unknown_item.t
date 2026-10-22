mod a {
    pub fn f() -> i64 { 1 }
}

fn main() { print(a::nope()); }
// error: `a` has no item `nope`

mod a {
    pub fn f() -> int { 1 }
}

fn main() { print(a::nope()); }
// error: `a` has no item `nope`

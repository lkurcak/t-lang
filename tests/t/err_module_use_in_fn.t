mod a {
    pub fn f() -> int { 1 }
}

fn main() {
    use a::f;
    print(f());
}
// error: a `use` can't be in a function

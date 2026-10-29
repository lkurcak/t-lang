// An alias of a struct is that struct: its fields are as private as
// they are.
mod m {
    pub struct S { n: i64 }
    pub type A = S;
    pub fn make() -> A { S { n: 1 } }
}

fn get(a: m::A) -> i64 { a.n }

fn main() { let _x = get(m::make()); }
// error: field `n` of `S` is private to `m`

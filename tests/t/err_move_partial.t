#[derive(Discard)]
struct Owner { a: Box<i64>, b: Box<i64> }

fn main() with {alloc, panic} {
    let o = Owner { a: Box::new(1), b: Box::new(2) };
    let a = o.a;
    print(*a);
    let _p = o;
}
// error: use of partly moved value `o`: `o.a` was moved out of it

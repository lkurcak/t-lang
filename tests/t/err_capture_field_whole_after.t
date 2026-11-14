// A closure holding `&mut p.x` lends a part of `p`: `p` as a whole can't
// be used while it lives.
#[derive(Discard)]
struct P { x: i64, y: i64 }

fn read(p: &P) -> i64 { p.y }

fn main() {
    let mut p = P { x: 1, y: 2 };
    let mut c = || p.x = 5;
    let _y = read(&p); // error: cannot borrow `p` while `p.x` is mutably borrowed
    c();
}

// `Send` and `Sync` are derived, like `Copy`, from the fields: what a
// reference shares is `Sync`, so `&T` is `Send` when `T` is `Sync`.
use core::cell::Cell;
use core::sync::Atomic;

fn send<T: Send>(_t: &T) {}
fn sync<T: Sync>(_t: &T) {}

#[derive(Discard, Send, Sync)]
struct Pair<'a, T> { a: T, r: &'a T }

#[derive(Discard, Send)]
struct Counter { n: Cell<u32> }

fn generic<T: Sync>(t: &T) {
    send(&t);
    sync(&t);
    let p = Pair { a: t, r: &t };
    send(&p);
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let c = Counter { n: Cell::new(0) };
    send(&c);
    let a = Atomic::new(1u64);
    sync(&a);
    send(&&a);
    let x = 5i64;
    let f = |y: i64| x + y;
    sync(&f);
    send(&f);
    let s = "hi";
    sync(&s);
    send(&[1u8, 2]);
    generic(&a);
    let g: fn(i64) -> i64 = |y| y;
    sync(&g);
    let mut out = os.stdout;
    out.line(&c.n.get())?; // expect: 0
    Result::Ok(())
}

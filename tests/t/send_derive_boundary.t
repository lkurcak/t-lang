// What it says is what it needs.
use core::cell::Cell;

#[derive(Discard, Send where T: Sync)]
export struct Holder<'a, T> { r: &'a T }

fn send<T: Send + Discard>(_t: T) {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 3u8;
    send(Holder { r: &n });
    let c = Cell::new(4u8);
    send(c);
    out.line(&n)?; // expect: 3
    Result::Ok(())
}

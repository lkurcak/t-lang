// The libraries' types are `Send` and `Sync` as their parts are.
use alloc::boxed::Box;
use alloc::list::List;
use alloc::string::String;

fn both<T: Send + Sync>(_t: &T) {}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut l: List<i64> = List::new();
    l.push(1);
    both(&l);
    both(&Box::new(3u8));
    both(&String::new());
    both(&Option::Some(1u8));
    let r: Result<u8, bool> = Result::Ok(1);
    both(&r);
    both(&(0..3u8));
    both(&os);
    let mut out = os.stdout;
    out.line(&l.len())?; // expect: 1
    Result::Ok(())
}

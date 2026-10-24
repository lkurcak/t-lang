// A string and a list are `Discard` now: each is freed where it goes out
// of scope, so `?` can return early while one is alive. The interpreter
// checks every buffer is freed, on the early path too.
use alloc::list::List;
use alloc::string::String;

fn sum_line(out: &mut std::io::Stdout, n: i64) -> Result<i64, std::io::Error> with {alloc, panic} {
    let mut l: List<i64> = List::new();
    let mut s = String::from("sum");
    let mut i = 0;
    while i < n {
        l.push(i);
        s.push('!');
        i = i + 1;
    }
    out.line(&s.as_str())?;
    let mut total = 0;
    for x in l.iter() {
        total = total + *x;
    }
    Result::Ok(total)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let t = sum_line(&mut out, 3)?; // expect: sum!!!
    out.line(&t) // expect: 3
}

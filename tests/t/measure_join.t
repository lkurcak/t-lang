// After an `if`, what each branch knows of a measure of a place it changed
// is kept, case by case.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    let first = l.is_empty();
    if first {
        l.push(1);
    } else {
        l.push(2);
        l.push(3);
    }
    out.line(&l[0])?; // expect: 1
    Result::Ok(())
}

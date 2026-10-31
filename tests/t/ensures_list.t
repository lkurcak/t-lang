// `List`'s methods say what they do to its length, so a caller knows it
// after: no check before indexing what was just pushed.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(1);
    l.push(2);
    l.push(3);
    out.line(&l[2])?; // expect: 3
    l.swap(0, 2);
    out.line(&l[2])?; // expect: 1
    l.pop();
    out.line(&l[1])?; // expect: 2
    l.truncate(1);
    l.push(4);
    out.line(&l.len())?; // expect: 2
    l.clear();
    l.push(5);
    out.line(&l[0])?; // expect: 5
    Result::Ok(())
}

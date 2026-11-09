// A method that takes `&mut self`, called on `x[i]`: that's `index_mut`'s
// place, as `x[i] = v` is, through fields and `*`s too (docs/59). One
// that takes `&self` is still `index`'s.
use alloc::list::List;

struct Row {
    items: List<u8>,
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<List<u8>> = List::new();
    l.push(List::new());
    l.push(List::new());
    l[0].push(5);
    l[1].push(6);
    l[1].push(7);
    let mut a: [List<u8>; 1] = [List::new()];
    a[0].push(8);
    let mut rows: List<Row> = List::new();
    rows.push(Row { items: List::new() });
    rows[0].items.push(9);
    let shared = &l;
    out.line(&shared[1].len())?;    // expect: 2
    out.line(&l[0][0])?;            // expect: 5
    out.line(&a[0][0])?;            // expect: 8
    out.line(&rows[0].items[0])?;   // expect: 9
    Ok(())
}

// A `for` over a list of linear values uses each one up, and when it runs
// to its end, its iterator holds none: `IntoIter<T>` is `Discard` where
// `len() == 0`, which `next` giving `None` says (docs/80).
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    l.push(Fd { n: 3 });
    let mut seen: List<i64> = List::new();
    for fd in l {
        let n = close(fd);
        if n == 2 {
            continue;
        }
        seen.push(n);
    }
    for n in &seen {
        out.line(n)?; // expect: 1
    }                 // expect: 3
    let mut m: List<Fd> = List::new();
    m.push(Fd { n: 4 });
    let mut total = 0;
    for fd in m {
        total = total + close(fd);
    }
    out.line(&total)?; // expect: 4
    let mut grid: List<List<Fd>> = List::new();
    let mut row: List<Fd> = List::new();
    row.push(Fd { n: 5 });
    grid.push(row);
    grid.push(List::new());
    let mut cells = 0;
    for row in grid {
        for fd in row {
            cells = cells + close(fd);
        }
    }
    out.line(&cells)?; // expect: 5
    Ok(())
}

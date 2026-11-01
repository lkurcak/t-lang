// What holds nothing, and what's empty: C has neither empty structs nor
// zero-length arrays, so the backend gives such a type a byte nothing
// reads, and points every empty slice somewhere real
// (docs/33-c-backend.md). The differential tests run it compiled too.
use alloc::boxed::Box;
use alloc::list::List;

#[derive(Discard, Copy, Eq)]
struct Nothing {}

#[derive(Discard)]
enum One { Only }

fn count<T>(s: &[T]) -> usize { s.len() }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let none: [i64; 0] = [];
    out.line(&none.len())?;                       // expect: 0
    out.line(&count(none.as_slice()))?;           // expect: 0
    let units = [(); 4];
    out.line(&units.len())?;                      // expect: 4
    let nothings = [Nothing {}, Nothing {}];
    out.line(&(nothings[0] == nothings[1]))?;     // expect: true
    let b = Box::new(());
    let () = *b;
    let n = Box::new(Nothing {});
    out.line(&(*n == Nothing {}))?;               // expect: true
    let o = One::Only;
    match o {
        One::Only => out.line(&"only")?,          // expect: only
    }
    out.line(&"".len())?;                         // expect: 0
    out.line(&b"".len())?;                        // expect: 0
    out.line(&("" == ""))?;                       // expect: true
    let empty: List<i64> = List::new();
    out.line(&empty.len())?;                      // expect: 0
    let mut sum = 0;
    for x in empty.iter() {
        sum = sum + *x;
    }
    out.line(&sum)?;                              // expect: 0
    let mut l: List<Nothing> = List::new();
    l.push(Nothing {});
    l.push(Nothing {});
    out.line(&l.len())?;                          // expect: 2
    let s = core::slice::empty::<i64>();
    match s.split_at(0) {
        Option::Some(parts) => out.line(&(parts.head.len() + parts.tail.len()))?, // expect: 0
        Option::None => out.line(&"none")?,
    }
    Result::Ok(())
}

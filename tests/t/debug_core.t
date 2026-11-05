// What core and alloc's `Debug` writes: Rust's `{:?}`, strings and
// characters quoted and escaped, values of slices, arrays and lists in
// brackets, variants by name.
use alloc::boxed::Box;
use alloc::list::List;
use alloc::string::String;
use core::cmp::Ordering;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let n = -128i8;
    let m = 18446744073709551615u64;
    let b = true;
    out.line(&f"{n:?} {m:?} {b:?}")?; // expect: -128 18446744073709551615 true
    let s = "tab\there \"quoted\" it's \\ \u{0}\u{1b}\u{7f} é";
    out.line(&f"{s:?}")?; // expect: "tab\there \"quoted\" it's \\ \0\u{1b}\u{7f} é"
    let q = '\'';
    let dq = '"';
    let nl = '\n';
    let e = 'é';
    out.line(&f"{q:?} {dq:?} {nl:?} {e:?}")?; // expect: '\'' '"' '\n' 'é'
    let u = ();
    out.line(&f"{u:?}")?; // expect: ()
    let a = [1u8, 2, 3];
    let sl = a.as_slice();
    let empty: [i32; 0] = [];
    out.line(&f"{a:?} {sl:?} {empty:?}")?; // expect: [1, 2, 3] [1, 2, 3] []
    let o: Option<&str> = Some("x");
    let none: Option<u8> = None;
    let r: Result<u8, Ordering> = Err(Ordering::Less);
    out.line(&f"{o:?} {none:?} {r:?}")?; // expect: Some("x") None Err(Less)
    let mut l = List::new();
    l.push(String::of(&"a\nb"));
    l.push(String::new());
    let bx = Box::new(Some(7i64));
    out.line(&f"{l:?} {bx:?}")?; // expect: ["a\nb", ""] Some(7)
    let range = 1u8..4;
    out.line(&f"{range:?}")?; // expect: Range { start: 1, end: 4 }
    let bad = "12x".parse::<u8>();
    out.line(&f"{bad:?}")?; // expect: Err(InvalidDigit)
    let mut five = 5i32;
    let refs = &&mut five;
    out.line(&f"{refs:?}")?; // expect: 5
    Ok(())
}

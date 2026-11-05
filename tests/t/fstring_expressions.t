// A hole holds any expression (step 44): an operator, a call, a method,
// an index, an `if`, a block, a path, a struct literal. `{`, `}` and `:`
// inside brackets of the hole's own, and `::`, are the expression's; a
// `:` outside them starts what's said of how it's written, as `:?`.
use alloc::string::String;

#[derive(Debug, Discard)]
struct Point { x: i64, y: i64 }

fn twice(n: i64) -> i64 with panic { n * 2 }

fn make() -> String with alloc { String::of(&"made") }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let p = Point { x: 3, y: -4 };
    let xs = [10i64, 20, 30];
    out.line(&f"{p.x + 1} {twice(p.y)} {xs[1]:?} {xs.len()}")?; // expect: 4 -8 20 3
    out.line(&f"{if p.x > 1 { '}' } else { '{' }} {{ {{ p.y }} }}")?; // expect: } { { p.y } }
    out.line(&f"{Point { x: p.y, y: p.x }:?} {Option::Some(p.x):?}")?; // expect: Point { x: -4, y: 3 } Some(3)
    out.line(&f"{{ let n = p.x * p.y; n - 1 }} = {{ let n = p.x * p.y; n - 1 }}")?; // expect: { let n = p.x * p.y; n - 1 } = { let n = p.x * p.y; n - 1 }
    out.line(&f"{ { let n = p.x * p.y; n - 1 } }")?; // expect: -13
    out.line(&f"[{make()}] {core::num::wrapping_add(p.x, 1)}")?; // expect: [made] 4
    out.line(&f"{':'} {(p.x + 62) as u8 as char} {-p.x}")?; // expect: : A -3
    Result::Ok(())
}

//! A generated family for floats in every place a value can be
//! (docs/101-floats.md, lemma F1: a float is its bits): a local, a
//! struct's field beside narrower and wider ones, a tuple's, an array's
//! and a slice's element, a variant's field, an `Option`, a generic
//! struct's field and a generic function's value, a closure's capture by
//! value and by reference, its argument and its result, a `Box`, a `List`,
//! a `Cell`, a write through `&mut`, a function value, a trait's method
//! through a bound, an opaque closure, a `Result`, a tuple in an `Option`,
//! a `swap`, and a const. Each takes a value in and gives it back out, of
//! both types, for zeros of both signs, a tenth, the least subnormal, the
//! greatest finite value, minus infinity and NaNs, quiet, signalling and
//! negative, each with a payload: nothing that only moves a float may
//! change a bit of it. Every program runs in the interpreter and, compiled
//! through C, natively, and both must give back every bit, NaNs' too.
//!
//! They're about what programs mean, the same on every platform, so they
//! run on Linux alone.

use std::fmt::Write as _;
use std::path::Path;

/// A program's output, run by the interpreter and, compiled, natively: they
/// must agree.
fn run_both(name: &str, src: &str) -> String {
    let checked = t_lang::check_source(src);
    let program = checked
        .result
        .unwrap_or_else(|e| panic!("{name}: {}\n{src}", e.render_in(&checked.sources)));
    let mut terminal = t_lang::interp::Captured::default();
    if let Err(e) = t_lang::interp::run(&program.lower(), &mut terminal) {
        panic!("{name}: the interpreter failed: {}", e.message);
    }
    let interp = String::from_utf8(terminal.out).unwrap();
    let read = |p: &Path| t_lang::no_files(p);
    let options = t_lang::cgen::Options {
        lines: false,
        target: t_lang::Target::Hosted,
    };
    let c = t_lang::build_c(Path::new("places.t"), src, &read, options)
        .result
        .unwrap_or_else(|e| panic!("{name}: didn't build: {}", e.message));
    let dir = std::env::temp_dir().join(format!("t-lang-float-places-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join(name);
    t_lang::cgen::compile(&c, &exe, t_lang::Target::Hosted)
        .unwrap_or_else(|e| panic!("{name}: didn't compile: {}", String::from(e)));
    let out = std::process::Command::new(&exe).output().unwrap();
    let _ = std::fs::remove_file(&exe);
    assert!(out.status.success(), "{name}: the program failed");
    let native = String::from_utf8(out.stdout).unwrap();
    assert_eq!(interp, native, "{name}: the interpreter and C disagree");
    native
}

/// A float type, the unsigned integer of its bits, and bit patterns worth
/// moving: `0.0`, `-0.0`, `0.1`, the least subnormal, the greatest finite,
/// `-inf`, a quiet NaN with a payload, a signalling one, and a negative
/// one.
struct Ty {
    name: &'static str,
    bits: &'static str,
    values: [u64; 9],
}

const F64: Ty = Ty {
    name: "f64",
    bits: "u64",
    values: [
        0,
        0x8000_0000_0000_0000,
        0x3FB9_9999_9999_999A,
        1,
        0x7FEF_FFFF_FFFF_FFFF,
        0xFFF0_0000_0000_0000,
        0x7FF8_0000_0000_0001,
        0x7FF0_0000_0000_0001,
        0xFFF8_0000_0000_0000,
    ],
};

const F32: Ty = Ty {
    name: "f32",
    bits: "u32",
    values: [
        0,
        0x8000_0000,
        0x3DCC_CCCD,
        1,
        0x7F7F_FFFF,
        0xFF80_0000,
        0x7FC0_0001,
        0x7F80_0001,
        0xFFC0_0000,
    ],
};

/// The places: each an arm of `via`, which puts `x` there and takes it
/// back out.
const PLACES: [(&str, &str); 23] = [
    ("local", "{ let y = x; y }"),
    ("field", "{ let m = Mixed { a: 1, x, b: 2, c: 3 }; m.x }"),
    ("tuple", "{ let t = (1u8, x, 2u64); t.1 }"),
    ("array", "{ let a = [x, x, x]; a[2] }"),
    ("variant", "match V::B(1, x) { V::B(_, y) => y, _ => x }"),
    ("option", "match Some(x) { Some(y) => y, None => x }"),
    ("generic field", "{ let w = W { t: x }; w.t }"),
    ("generic value", "id(x)"),
    ("capture", "{ let c = || x; c() }"),
    ("capture by reference", "{ let r = &x; let c = || *r; c() }"),
    ("closure argument", "{ let c = |y: F| y; c(x) }"),
    ("box", "{ let b = Box::new(x); *b }"),
    (
        "list",
        "{ let mut l: List<F> = List::new(); l.push(x); l.as_slice()[0] }",
    ),
    ("cell", "{ let c = Cell::new(x); c.set(x); c.get() }"),
    ("slice", "{ let a = [x, x]; let s: &[F] = &a; s[1] }"),
    (
        "through &mut",
        "{ let mut y = x; let r = &mut y; *r = x; y }",
    ),
    ("function value", "{ let f: fn(F) -> F = get; f(x) }"),
    ("trait method", "by_bound(&G { x })"),
    ("opaque closure", "made(x)()"),
    (
        "result",
        "{ let r: Result<F, u8> = Ok(x); match r { Ok(y) => y, Err(_) => x } }",
    ),
    (
        "nested",
        "match Some((1u8, W { t: Mixed { a: 1, x, b: 2, c: 3 } })) { Some((_, w)) => w.t.x, None => x }",
    ),
    (
        "swap",
        "{ let mut a = x; let mut b = x; core::mem::swap(&mut a, &mut b); a }",
    ),
    ("const", ""),
];

/// The program for `ty`: every place, every value, each line `place bits`.
fn program(ty: &Ty) -> String {
    let (f, b) = (ty.name, ty.bits);
    let mut arms = String::new();
    for (k, (_, arm)) in PLACES.iter().enumerate() {
        if !arm.is_empty() {
            writeln!(arms, "        {k} => {arm},").unwrap();
        }
    }
    let mut consts = String::new();
    let mut shown = String::new();
    for (i, v) in ty.values.iter().enumerate() {
        writeln!(consts, "const K{i}: F = {f}::from_bits({v:#x});").unwrap();
        writeln!(
            shown,
            "    out.line(&f\"{} {{K{i}.to_bits()}}\")?;",
            PLACES.len() - 1
        )
        .unwrap();
    }
    let values: Vec<String> = ty.values.iter().map(|v| format!("{v:#x}")).collect();
    let n = ty.values.len();
    let places = PLACES.len() - 1;
    format!(
        "use alloc::boxed::Box;
use alloc::list::List;
use core::cell::Cell;

type F = {f};

#[derive(Copy, Discard)]
struct Mixed {{ a: u8, x: F, b: u16, c: u64 }}

#[derive(Copy, Discard)]
enum V {{ A(u8), B(u8, F), C(u64), D }}

#[derive(Copy, Discard)]
struct W<T> {{ t: T }}

trait Give {{ fn give(&self) -> F; }}

#[derive(Copy, Discard)]
struct G {{ x: F }}

impl Give for G {{ fn give(&self) -> F {{ self.x }} }}

fn id<T>(t: T) -> T {{ t }}

fn by_bound<T: Give>(t: &T) -> F {{ t.give() }}

fn made(x: F) -> impl Fn() -> F + Discard {{ move || x }}

fn get(x: F) -> F {{ x }}

#[allow(dead_code)]
fn unused() -> u8 {{
    let _ = V::A(0);
    let _ = V::C(0);
    let _ = V::D;
    0
}}

fn via(k: u8, x: F) -> F with alloc {{
    match k {{
{arms}        _ => x,
    }}
}}

{consts}
fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {{
    let mut out = os.stdout;
    let bits: [{b}; {n}] = [{values}];
    for k in 0..{places}u8 {{
        for i in 0..{n} {{
            out.line(&f\"{{k}} {{via(k, {f}::from_bits(bits[i])).to_bits()}}\")?;
        }}
    }}
{shown}    Ok(())
}}
",
        values = values.join(", ")
    )
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn floats_keep_their_bits_in_every_place() {
    let mut wrong = Vec::new();
    for ty in [F64, F32] {
        let src = program(&ty);
        let out = run_both(ty.name, &src);
        let mut lines = out.lines();
        let mut expect = |place: usize, v: u64| {
            let want = format!("{place} {v}");
            match lines.next() {
                Some(got) if got == want => {}
                got => wrong.push(format!(
                    "{}, {}: {v:#x} came back as {got:?}",
                    ty.name, PLACES[place].0
                )),
            }
        };
        for place in 0..PLACES.len() - 1 {
            for &v in &ty.values {
                expect(place, v);
            }
        }
        for &v in &ty.values {
            expect(PLACES.len() - 1, v);
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

//! Generated families for floats (docs/101-floats.md), each with an oracle
//! of its own: Rust's `f64` and `f32`, which round as IEEE 754 says, and
//! its `{:?}`, `{:.n}` and `parse`, which core's `Display`, `Fixed` and
//! `Parse` match. Every program runs in the interpreter and, compiled
//! through C, natively, and both must give exactly what Rust does: the
//! same bits of every result, but for a NaN's, which aren't said.
//!
//! - text: random and boundary bit patterns of both types, each written
//!   the shortest way and with 0 to 20 digits after the point, and read
//!   back (lemma F6);
//! - parsing: random decimals, ones exactly halfway between two floats and
//!   just past that, and very long ones;
//! - arithmetic: every operator and method of `Float` over pairs of values
//!   that include zeros of both signs, subnormals, infinities and NaNs
//!   (lemma F2, F7);
//! - conversions: `as` between each integer type and each float type, both
//!   ways, at the boundaries: the same number, or a panic (lemma F5).
//!
//! They're about what programs mean, the same on every platform, so they
//! run on Linux alone.

use std::fmt::Write as _;
use std::path::Path;

/// A program's output, run by the interpreter and, compiled, natively: they
/// must agree, and that's what's checked against the oracle.
fn run_both(name: &str, src: &str) -> String {
    run(name, src, true)
}

/// A program's output, compiled and run natively, and by the interpreter
/// too if `interpret`: it works out big numbers in hundreds of steps of
/// its own, so many values are for the machine alone.
fn run(name: &str, src: &str, interpret: bool) -> String {
    let checked = t_lang::check_source(src);
    let program = checked
        .result
        .unwrap_or_else(|e| panic!("{name}: {}\n{src}", e.render_in(&checked.sources)));
    let (interp, panicked) = match interpret {
        true => {
            let mut terminal = t_lang::interp::Captured::default();
            let result = t_lang::interp::run(&program.lower(), &mut terminal);
            let panicked = match result {
                Ok(_) => String::new(),
                Err(e) => e.message.clone(),
            };
            (Some(String::from_utf8(terminal.out).unwrap()), panicked)
        }
        false => (None, String::new()),
    };
    let read = |p: &Path| t_lang::no_files(p);
    let options = t_lang::cgen::Options {
        lines: false,
        target: t_lang::Target::Hosted,
    };
    let c = t_lang::build_c(Path::new("float.t"), src, &read, options)
        .result
        .unwrap_or_else(|e| panic!("{name}: didn't build: {}", e.message));
    let dir = std::env::temp_dir().join(format!("t-lang-floats-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join(name);
    t_lang::cgen::compile(&c, &exe, t_lang::Target::Hosted)
        .unwrap_or_else(|e| panic!("{name}: didn't compile: {}", String::from(e)));
    let out = std::process::Command::new(&exe).output().unwrap();
    let _ = std::fs::remove_file(&exe);
    let native = String::from_utf8(out.stdout).unwrap();
    if let Some(interp) = &interp {
        assert_eq!(interp, &native, "{name}: the interpreter and C disagree");
    }
    let native_err = String::from_utf8_lossy(&out.stderr);
    if !panicked.is_empty() {
        assert!(
            native_err.contains(&panicked),
            "{name}: the interpreter panicked with {panicked:?}, C wrote {native_err:?}"
        );
    }
    native + &panicked
}

/// A generator of pseudo-random numbers, the same in every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // SplitMix64.
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Bit patterns of `f64`s worth trying: the boundaries of the range of
/// every `step`th exponent, zeros, subnormals, and random ones; no NaNs.
fn f64_bits(rng: &mut Rng, random: usize, step: usize) -> Vec<u64> {
    let mut out = vec![0, 1, 2, 0x000F_FFFF_FFFF_FFFF, 0x0010_0000_0000_0000];
    out.extend([
        0x7FEF_FFFF_FFFF_FFFF,
        0x3FF0_0000_0000_0000,
        0x4340_0000_0000_0000,
    ]);
    for e in (0..2047u64).step_by(step) {
        out.extend([e << 52, (e << 52) | 1, (e << 52) | 0x000F_FFFF_FFFF_FFFF]);
    }
    for _ in 0..random {
        let b = rng.next();
        if (b >> 52) & 0x7FF != 0x7FF {
            out.push(b);
        }
    }
    out
}

/// The same, of `f32`s.
fn f32_bits(rng: &mut Rng, random: usize, step: usize) -> Vec<u32> {
    let mut out = vec![0, 1, 2, 0x007F_FFFF, 0x0080_0000, 0x7F7F_FFFF, 0x3F80_0000];
    for e in (0..255u32).step_by(step) {
        out.extend([e << 23, (e << 23) | 1, (e << 23) | 0x007F_FFFF]);
    }
    for _ in 0..random {
        let b = rng.next() as u32;
        if (b >> 23) & 0xFF != 0xFF {
            out.push(b);
        }
    }
    out
}

/// `[a, b, ..]` as a T array literal of type `[t; n]`.
fn array<T: std::fmt::Display>(t: &str, xs: &[T]) -> String {
    let items: Vec<String> = xs.iter().map(|x| x.to_string()).collect();
    format!("[{t}; {}] = [{}]", xs.len(), items.join(", "))
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_float_text() {
    text(101, (8, 409), (6, 64), true);
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_float_text_native() {
    text(102, (3000, 1), (2000, 1), false);
}

/// Floats as text: of each type, how many random ones, and of every how
/// many exponents the boundaries; run by the interpreter too if
/// `interpret`. The program makes the same bits as `f64_bits` and
/// `f32_bits`, by the same generator, rather than reading thousands of
/// literals, which take the checker long.
fn text(seed: u64, f64s: (usize, usize), f32s: (usize, usize), interpret: bool) {
    let mut rng = Rng(seed);
    let b64 = f64_bits(&mut rng, f64s.0, f64s.1);
    let b32 = f32_bits(&mut rng, f32s.0, f32s.1);
    let src = format!(
        "use core::num::{{wrap, wrapping_add, wrapping_mul}};

fn next(s: &mut u64) -> u64 {{
    *s = wrapping_add(*s, 0x9E37_79B9_7F4A_7C15);
    let mut z = *s;
    z = wrapping_mul(z ^ (z >> 30), 0xBF58_476D_1CE4_E5B9);
    z = wrapping_mul(z ^ (z >> 27), 0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}}

fn show64<W: core::io::Write>(out: &mut W, b: u64) -> Result<(), core::io::Error> with {{alloc, W::e}} {{
    let x = f64::from_bits(b);
    let s = alloc::string::String::of(&x);
    let back = match s.as_str().parse::<f64>() {{ Ok(y) => y.to_bits() == b, Err(_) => false }};
    let p = (b % 21) as usize;
    out.line(&f\"{{s.as_str()}} {{back}} {{x:.3}} {{-x:.0}} {{core::fmt::fixed(&x, p)}}\")
}}

fn show32<W: core::io::Write>(out: &mut W, b: u32) -> Result<(), core::io::Error> with {{alloc, W::e}} {{
    let x = f32::from_bits(b);
    let s = alloc::string::String::of(&x);
    let back = match s.as_str().parse::<f32>() {{ Ok(y) => y.to_bits() == b, Err(_) => false }};
    out.line(&f\"{{s.as_str()}} {{back}} {{x:.5}}\")
}}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {{
    let mut out = os.stdout;
    let mut s: u64 = {seed};
    let fixed64: [u64; 8] = [0, 1, 2, 0x000F_FFFF_FFFF_FFFF, 0x0010_0000_0000_0000, 0x7FEF_FFFF_FFFF_FFFF, 0x3FF0_0000_0000_0000, 0x4340_0000_0000_0000];
    for b in fixed64 {{
        show64(&mut out, b)?;
    }}
    let mut e: u64 = 0;
    while e < 2047 {{
        show64(&mut out, e << 52)?;
        show64(&mut out, (e << 52) | 1)?;
        show64(&mut out, (e << 52) | 0x000F_FFFF_FFFF_FFFF)?;
        e = e + {s64};
    }}
    for _ in 0..{r64}u32 {{
        let b = next(&mut s);
        if (b >> 52) & 0x7FF != 0x7FF {{
            show64(&mut out, b)?;
        }}
    }}
    let fixed32: [u32; 7] = [0, 1, 2, 0x007F_FFFF, 0x0080_0000, 0x7F7F_FFFF, 0x3F80_0000];
    for b in fixed32 {{
        show32(&mut out, b)?;
    }}
    let mut e: u32 = 0;
    while e < 255 {{
        show32(&mut out, e << 23)?;
        show32(&mut out, (e << 23) | 1)?;
        show32(&mut out, (e << 23) | 0x007F_FFFF)?;
        e = e + {s32};
    }}
    for _ in 0..{r32}u32 {{
        let b: u32 = wrap(next(&mut s));
        if (b >> 23) & 0xFF != 0xFF {{
            show32(&mut out, b)?;
        }}
    }}
    Ok(())
}}
",
        r64 = f64s.0,
        s64 = f64s.1,
        r32 = f32s.0,
        s32 = f32s.1,
    );
    let mut want = String::new();
    for b in &b64 {
        let x = f64::from_bits(*b);
        let p = (b % 21) as usize;
        writeln!(want, "{x:?} true {x:.3} {:.0} {x:.p$}", -x).unwrap();
    }
    for b in &b32 {
        let x = f32::from_bits(*b);
        writeln!(want, "{x:?} true {x:.5}").unwrap();
    }
    let got = run(&format!("text{seed}"), &src, interpret);
    for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
        assert_eq!(g, w, "line {i}");
    }
    assert_eq!(got.lines().count(), want.lines().count());
}

/// The exact decimal of `n · 2^e`: big integers, by hand, as a test has no
/// crate for them.
fn exact_decimal(n: u128, e: i32) -> String {
    // Digits, least significant first, base 10^9.
    let mut d: Vec<u64> = Vec::new();
    let mut x = n;
    while x > 0 {
        d.push((x % 1_000_000_000) as u64);
        x /= 1_000_000_000;
    }
    let times = |d: &mut Vec<u64>, k: u64| {
        let mut carry = 0;
        for limb in d.iter_mut() {
            let t = *limb * k + carry;
            *limb = t % 1_000_000_000;
            carry = t / 1_000_000_000;
        }
        while carry > 0 {
            d.push(carry % 1_000_000_000);
            carry /= 1_000_000_000;
        }
    };
    // n · 2^e = n · 2^e, or n · 5^-e / 10^-e.
    let shift = if e >= 0 {
        for _ in 0..e {
            times(&mut d, 2);
        }
        0
    } else {
        for _ in 0..-e {
            times(&mut d, 5);
        }
        (-e) as usize
    };
    let mut s = String::new();
    for (i, limb) in d.iter().rev().enumerate() {
        match i {
            0 => write!(s, "{limb}").unwrap(),
            _ => write!(s, "{limb:09}").unwrap(),
        }
    }
    if s.is_empty() {
        s.push('0');
    }
    if shift == 0 {
        return s;
    }
    if s.len() <= shift {
        s = "0".repeat(shift - s.len() + 1) + &s;
    }
    let (whole, frac) = s.split_at(s.len() - shift);
    format!("{whole}.{frac}")
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_float_parse() {
    parse(202, 1, true);
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_float_parse_native() {
    parse(203, 20, false);
}

/// Text read as floats: `times` as many random strings as the least, run
/// by the interpreter too if `interpret`.
fn parse(seed: u64, times: usize, interpret: bool) {
    let mut rng = Rng(seed);
    let mut strs: Vec<String> = Vec::new();
    // Exactly halfway between two floats, of both types, and a little
    // past that: the hardest to round.
    for _ in 0..6 * times {
        let b = rng.next() % 0x7FE0_0000_0000_0000 + 1;
        let exp = ((b >> 52) & 0x7FF) as i32;
        let m = (b & 0x000F_FFFF_FFFF_FFFF) | if exp > 0 { 1 << 52 } else { 0 };
        let e = if exp > 0 { exp - 1075 } else { -1074 };
        let mid = exact_decimal(2 * u128::from(m) + 1, e - 1);
        strs.push(mid.clone());
        strs.push(format!("{mid}00000000000000000000001"));
    }
    for _ in 0..5 * times {
        let b = (rng.next() as u32) % 0x7F00_0000 + 1;
        let exp = ((b >> 23) & 0xFF) as i32;
        let m = (b & 0x007F_FFFF) | if exp > 0 { 1 << 23 } else { 0 };
        let e = if exp > 0 { exp - 150 } else { -149 };
        strs.push(exact_decimal(2 * u128::from(m) + 1, e - 1));
    }
    for _ in 0..30 * times {
        let n = (rng.next() % 30 + 1) as usize;
        let digits: String = (0..n)
            .map(|_| char::from(b'0' + (rng.next() % 10) as u8))
            .collect();
        let point = (rng.next() % (n as u64 + 1)) as usize;
        let mut s = format!("{}.{}", &digits[..point], &digits[point..]);
        if s.starts_with('.') {
            s.insert(0, '0');
        }
        if !rng.next().is_multiple_of(3) {
            write!(s, "e{}", rng.next() as i64 % 340).unwrap();
        }
        if rng.next().is_multiple_of(4) {
            s.insert(0, '-');
        }
        strs.push(s);
    }
    strs.extend(
        [
            "0",
            "-0",
            "1e400",
            "1e-400",
            "2.4703282292062327e-324",
            "2.4703282292062328e-324",
            "1.7976931348623158e308",
            "1.7976931348623159e308",
            "3.4028235677973366e38",
            "7.006492321624085e-46",
            "7.006492321624086e-46",
            "inf",
            "-Infinity",
            "nan",
            "+1.5",
            ".5",
            "1.",
            "1E5",
            "1e",
            "e5",
            "",
            "-",
            "1.2.3",
            "0x10",
            "1e+",
            "--1",
            "1e1000000",
        ]
        .map(String::from),
    );
    strs.push(format!("{}e-900", "9".repeat(900)));
    strs.push(format!("0.{}1e400", "0".repeat(399)));
    let mut src = String::from("use core::io::Write;\n\n");
    src.push_str(
        "fn main(os: std::Os) -> Result<(), std::io::Error> {\n    let mut out = os.stdout;\n",
    );
    let quoted: Vec<String> = strs.iter().map(|s| format!("\"{s}\"")).collect();
    writeln!(
        src,
        "    let strs: [&str; {}] = [{}];",
        strs.len(),
        quoted.join(", ")
    )
    .unwrap();
    src.push_str(
        "    for s in strs {
        show(&mut out, s)?;
    }
    Ok(())
}

fn show<W: Write>(out: &mut W, s: &str) -> Result<(), core::io::Error> with W::e {
    match s.parse::<f64>() {
        Ok(x) => if x.is_nan() { out.put(\"nan\")?; } else { out.put(&x.to_bits())?; },
        Err(_) => out.put(\"err\")?,
    }
    out.put(\" \")?;
    match s.parse::<f32>() {
        Ok(x) => if x.is_nan() { out.put(\"nan\")?; } else { out.put(&x.to_bits())?; },
        Err(_) => out.put(\"err\")?,
    }
    out.put(\"\\n\")
}
",
    );
    let mut want = String::new();
    for s in &strs {
        let a = match s.parse::<f64>() {
            Ok(x) if x.is_nan() => "nan".to_string(),
            Ok(x) => x.to_bits().to_string(),
            Err(_) => "err".to_string(),
        };
        let b = match s.parse::<f32>() {
            Ok(x) if x.is_nan() => "nan".to_string(),
            Ok(x) => x.to_bits().to_string(),
            Err(_) => "err".to_string(),
        };
        writeln!(want, "{a} {b}").unwrap();
    }
    let got = run(&format!("parse{seed}"), &src, interpret);
    for ((g, w), s) in got.lines().zip(want.lines()).zip(&strs) {
        assert_eq!(g, w, "{s:?}");
    }
    assert_eq!(got.lines().count(), want.lines().count());
}

/// IEEE 754-2019's `maximum`, which `Float::max` is: a NaN if either is,
/// and `0.0` of the zeros. Rust's `max` drops a NaN instead.
fn maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { b } else { a }
    } else {
        a.max(b)
    }
}

/// And `minimum`.
fn minimum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else {
        a.min(b)
    }
}

/// A float's bits, or `nan` for a NaN, whose bits aren't said.
fn bits(x: f64) -> String {
    match x.is_nan() {
        true => "nan".to_string(),
        false => x.to_bits().to_string(),
    }
}

fn bits32(x: f32) -> String {
    match x.is_nan() {
        true => "nan".to_string(),
        false => x.to_bits().to_string(),
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_float_arithmetic() {
    let mut rng = Rng(303);
    let mut xs = f64_bits(&mut rng, 40, 37);
    xs.extend([
        0x8000_0000_0000_0000,
        0x7FF0_0000_0000_0000,
        0xFFF0_0000_0000_0000,
    ]);
    xs.extend([
        0x7FF8_0000_0000_0000,
        0x3FE0_0000_0000_0000,
        0xBFF8_0000_0000_0000,
    ]);
    xs.truncate(90);
    let mut ys = xs.clone();
    ys.rotate_left(17);
    let ops = "a + b, a - b, a * b, a / b, -a, a.sqrt(), a.floor(), a.ceil(), a.trunc(), \
               a.round(), a.min(b), a.max(b), a.abs(), a.copysign(b)";
    let n = ops.split(", ").count();
    let mut src = String::from("fn main(os: std::Os) -> Result<(), std::io::Error> {\n");
    src.push_str("    let mut out = os.stdout;\n");
    writeln!(src, "    let xs: {};", array("u64", &xs)).unwrap();
    writeln!(src, "    let ys: {};", array("u64", &ys)).unwrap();
    src.push_str("    for i in 0..xs.len() {\n");
    src.push_str("        let (a, b) = (f64::from_bits(xs[i]), f64::from_bits(ys[i]));\n");
    writeln!(src, "        let r: [f64; {n}] = [{ops}];").unwrap();
    src.push_str("        for x in r {\n            if x.is_nan() { out.put(\"nan \")?; } else { out.put(&x.to_bits())?; out.put(\" \")?; }\n        }\n");
    src.push_str("        let (c, d) = (a as f32, b as f32);\n");
    src.push_str("        out.line(&f\"{(a < b)} {(a == b)} {(a >= b)} {c.to_bits()} {(c * d).to_bits()} {(c / d).to_bits()}\")?;\n");
    src.push_str("    }\n    Ok(())\n}\n");
    // `as f32` panics unless exact: only the `f32`s of the `f64`s are.
    let src = src.replace(
        "a as f32, b as f32",
        "f32::nearest_f64(a), f32::nearest_f64(b)",
    );
    let mut want = String::new();
    for (x, y) in xs.iter().zip(&ys) {
        let (a, b) = (f64::from_bits(*x), f64::from_bits(*y));
        let r = [
            a + b,
            a - b,
            a * b,
            a / b,
            -a,
            a.sqrt(),
            a.floor(),
            a.ceil(),
            a.trunc(),
            a.round(),
            minimum(a, b),
            maximum(a, b),
            a.abs(),
            a.copysign(b),
        ];
        for v in r {
            write!(want, "{} ", bits(v)).unwrap();
        }
        let (c, d) = (a as f32, b as f32);
        let c32 = |x: f32| match x.is_nan() {
            true => "nan".to_string(),
            false => x.to_bits().to_string(),
        };
        let _ = bits32;
        writeln!(
            want,
            "{} {} {} {} {} {}",
            a < b,
            a == b,
            a >= b,
            c32(c),
            c32(c * d),
            c32(c / d)
        )
        .unwrap();
    }
    let got = run_both("arithmetic", &src);
    // A NaN's bits aren't said, so `to_bits` of one isn't compared.
    let normal = |s: &str| -> String {
        s.split(' ')
            .map(|w| match w.parse::<u64>() {
                Ok(b) if f32::from_bits(b as u32).is_nan() && b < 1 << 32 && b > 0x7F80_0000 => {
                    "nan".to_string()
                }
                _ => w.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
        assert_eq!(normal(g), normal(w), "case {i}: {:x} {:x}", xs[i], ys[i]);
    }
    assert_eq!(got.lines().count(), want.lines().count());
}

/// Integer types, by name, with their least and greatest values.
const INTS: [(&str, i128, i128); 8] = [
    ("i8", i8::MIN as i128, i8::MAX as i128),
    ("i16", i16::MIN as i128, i16::MAX as i128),
    ("i32", i32::MIN as i128, i32::MAX as i128),
    ("i64", i64::MIN as i128, i64::MAX as i128),
    ("u8", 0, u8::MAX as i128),
    ("u16", 0, u16::MAX as i128),
    ("u32", 0, u32::MAX as i128),
    ("u64", 0, u64::MAX as i128),
];

/// Whether integer `n` is a float of `p` bits of precision exactly.
fn representable(n: i128, p: u32) -> bool {
    let a = n.unsigned_abs();
    a == 0 || {
        let width = 128 - a.leading_zeros();
        width <= p || a.trailing_zeros() >= width - p
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_float_conversions() {
    // Each conversion that panics ends its program, so one of each kind is
    // a program of its own, per pair of types; those that don't share one.
    for (k, (int, lo, hi)) in INTS.into_iter().enumerate() {
        for (float, p) in [("f32", 24u32), ("f64", 53)] {
            let mut values: Vec<i128> = vec![lo, hi, 0, 1, -1, lo + 1, hi - 1];
            for b in [p - 1, p, p + 1, 31, 63] {
                let v = 1i128 << b;
                values.extend([v - 1, v, v + 1, -v - 1, -v, -v + 1]);
            }
            values.retain(|v| (lo..=hi).contains(v));
            values.sort();
            values.dedup();
            let (exact, inexact): (Vec<i128>, Vec<i128>) =
                values.iter().partition(|v| representable(**v, p));
            // From the float: the same integer, where it's one of the
            // type's, or a panic.
            let wide = |x: f64| match float {
                "f32" => (x as f32) as f64,
                _ => x,
            };
            let is_int =
                |x: f64| x.is_finite() && x.trunc() == x && (x as i128) >= lo && (x as i128) <= hi;
            let probes = [
                lo as f64,
                hi as f64,
                lo as f64 - 1.0,
                0.5,
                -0.0,
                f64::NAN,
                f64::INFINITY,
            ];
            let lit = |x: f64| match format!("{x:?}").as_str() {
                "NaN" => format!("{float}::nan()"),
                "inf" => format!("{float}::infinity()"),
                t => format!("{t}{float}"),
            };
            let (fits, fails): (Vec<f64>, Vec<f64>) =
                probes.into_iter().map(wide).partition(|x| is_int(*x));
            let mut src = String::from(
                "fn main(os: std::Os) -> Result<(), std::io::Error> with panic {\n    let mut out = os.stdout;\n",
            );
            writeln!(src, "    let ns: {};", array(int, &exact)).unwrap();
            writeln!(
                src,
                "    for n in ns {{\n        let x = n as {float};\n        out.line(&f\"{{x}} {{x as {int} == n}}\")?;\n    }}"
            )
            .unwrap();
            for x in &fits {
                writeln!(src, "    out.line(&({} as {int}))?;", lit(*x)).unwrap();
            }
            src.push_str("    Ok(())\n}\n");
            let mut want = String::new();
            for n in &exact {
                match float {
                    "f32" => writeln!(want, "{:?} true", *n as f32).unwrap(),
                    _ => writeln!(want, "{:?} true", *n as f64).unwrap(),
                }
            }
            for x in &fits {
                writeln!(want, "{}", *x as i128).unwrap();
            }
            let name = format!("{int}_{float}");
            assert_eq!(run_both(&name, &src), want, "{name}");
            // To the float, an integer that isn't one.
            if let Some(n) = inexact.get(k % inexact.len().max(1)) {
                let src = format!(
                    "fn main() with panic {{\n    let n: {int} = {n};\n    let _ = n as {float};\n}}\n"
                );
                let name = format!("{int}_{float}_inexact");
                let got = run_both(&name, &src);
                assert!(
                    got.contains(&format!("isn't an `{float}` exactly")),
                    "{name}: {got:?}"
                );
            }
            // From the float, one that isn't an integer of the type.
            let x = fails[k % fails.len()];
            let src = format!(
                "fn main() with panic {{\n    let x: {float} = {};\n    let _ = x as {int};\n}}\n",
                lit(x)
            );
            let name = format!("{float}_{int}_fails");
            let got = run_both(&name, &src);
            assert!(
                got.contains(&format!("`as` failed: the `{float}` isn't")),
                "{name}: {got:?}"
            );
        }
    }
}

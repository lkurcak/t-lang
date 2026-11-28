//! The math functions of `Float` (docs/102-math.md, docs/103-more-math.md)
//! against MPFR's answers, which `tests/math/vectors.txt` holds: for each
//! function of each type, the inputs MPFR found hardest to round among
//! millions, which go the slow way, random ones, the edges, the
//! infinities and a NaN too, and what no random draw finds, as an input in
//! each interval of `atan`'s table (`scripts/math/vectors.c`). Each
//! function's are run in the interpreter and, compiled through C,
//! natively, which must agree, bit for bit, and give MPFR's answer: the
//! same bits, but for a NaN's, which aren't said.
//!
//! The check of every `f32` input and of millions of `f64` ones needs MPFR,
//! so isn't here: `scripts/math/check.sh` runs it.
//!
//! They're about what programs mean, the same on every platform, so they
//! run on Linux alone.

use std::collections::BTreeMap;
use std::path::Path;

/// A program's output, run by the interpreter and, compiled, natively,
/// which must agree.
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
    let c = t_lang::build_c(Path::new("math.t"), src, &read, options)
        .result
        .unwrap_or_else(|e| panic!("{name}: didn't build: {}", e.message));
    let dir = std::env::temp_dir().join(format!("t-lang-math-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join(name);
    t_lang::cgen::compile(&c, &exe, t_lang::Target::Hosted)
        .unwrap_or_else(|e| panic!("{name}: didn't compile: {}", String::from(e)));
    let out = std::process::Command::new(&exe).output().unwrap();
    let _ = std::fs::remove_file(&exe);
    let native = String::from_utf8(out.stdout).unwrap();
    assert_eq!(interp, native, "{name}: the interpreter and C disagree");
    native
}

/// One input and MPFR's answer, as bits.
struct Case {
    x: u64,
    y: u64,
    want: u64,
}

/// The cases of each function of each type, by `(name, type)`.
fn vectors() -> BTreeMap<(String, String), Vec<Case>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/math/vectors.txt");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut out: BTreeMap<(String, String), Vec<Case>> = BTreeMap::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
        let case = match w.len() {
            4 => Case {
                x: hex(w[2]),
                y: 0,
                want: hex(w[3]),
            },
            5 => Case {
                x: hex(w[2]),
                y: hex(w[3]),
                want: hex(w[4]),
            },
            _ => panic!("a line of {} isn't a case: {line:?}", path.display()),
        };
        out.entry((w[0].to_string(), w[1].to_string()))
            .or_default()
            .push(case);
    }
    out
}

fn is_nan(b: u64, ty: &str) -> bool {
    match ty {
        "f64" => b & 0x7FFF_FFFF_FFFF_FFFF > 0x7FF0_0000_0000_0000,
        _ => b & 0x7FFF_FFFF > 0x7F80_0000,
    }
}

/// Each function of `ty` gives MPFR's answer for each of its cases. The
/// functions run side by side, as the slow paths take the interpreter a
/// while: a function's cases are one program.
fn check(ty: &str) {
    let all: Vec<((String, String), Vec<Case>)> = vectors()
        .into_iter()
        .filter(|((_, t), _)| t == ty)
        .collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut wrong: Vec<(usize, Vec<String>)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads.min(all.len()))
            .map(|_| {
                s.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(((name, _), cases)) = all.get(i) else {
                            return done;
                        };
                        done.push((i, check_one(name, ty, cases)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect()
    });
    wrong.sort_by_key(|(i, _)| *i);
    let wrong: Vec<String> = wrong.into_iter().flat_map(|(_, w)| w).collect();
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The cases of `name` of `ty` that don't give MPFR's answer.
fn check_one(name: &str, ty: &str, cases: &[Case]) -> Vec<String> {
    let bits = if ty == "f64" { "u64" } else { "u32" };
    let mut wrong = Vec::new();
    let two = matches!(name, "pow" | "atan2" | "hypot");
    let xs: Vec<String> = cases.iter().map(|c| c.x.to_string()).collect();
    let ys: Vec<String> = cases.iter().map(|c| c.y.to_string()).collect();
    let n = cases.len();
    let call = match two {
        true => format!("{ty}::from_bits(XS[i]).{name}({ty}::from_bits(YS[i]))"),
        false => format!("{ty}::from_bits(XS[i]).{name}()"),
    };
    let src = format!(
        "const XS: [{bits}; {n}] = [{}];\n\
         const YS: [{bits}; {n}] = [{}];\n\
         fn main(os: std::Os) -> Result<(), std::io::Error> {{\n\
         \x20   let mut out = os.stdout;\n\
         \x20   for i in 0..{n}usize {{\n\
         \x20       out.line(&{call}.to_bits())?;\n\
         \x20   }}\n\
         \x20   Ok(())\n\
         }}\n",
        xs.join(", "),
        ys.join(", "),
    );
    let out = run_both(&format!("{name}_{ty}"), &src);
    let got: Vec<u64> = out.lines().map(|l| l.parse().unwrap()).collect();
    assert_eq!(got.len(), n, "{name} of {ty}: not every case printed");
    for (c, g) in cases.iter().zip(got) {
        if g != c.want && !(is_nan(g, ty) && is_nan(c.want, ty)) {
            wrong.push(format!(
                "{name}({:#x}{}) of {ty} = {g:#x}, not {:#x}",
                c.x,
                if two {
                    format!(", {:#x}", c.y)
                } else {
                    String::new()
                },
                c.want
            ));
        }
    }
    wrong
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn math_f64_as_mpfr() {
    check("f64");
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn math_f32_as_mpfr() {
    check("f32");
}

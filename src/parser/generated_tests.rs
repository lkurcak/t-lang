//! Small, deterministic syntax properties, run by ordinary `cargo test`.
//! No type checking, libraries, external fuzz runner, or random seed from
//! the environment: failures name the seed, case/mutation, and exact input.

use super::{parse, parse_for_editor};
use crate::ast::{Program, ROOT};
use crate::error::{Error, Phase, Span};
use crate::fmt::{self, FmtError};
use crate::lexer::{self, Tok};
use std::panic::{UnwindSafe, catch_unwind};

const SEEDS: [u64; 2] = [0x7465_7374_5f54, 0xc0ff_ee12_3456_789a];
const PROGRAMS_PER_SEED: usize = 32;
const MAX_DEPTH: usize = 3;
const MAX_BYTES: usize = 8_192;
const DELIMITERS_PER_PROGRAM: usize = 16;

/// An explicitly specified PRNG so the corpus is stable across platforms
/// and dependency updates. Wrapping arithmetic is intentional.
struct Generator(u64);

impl Generator {
    fn pick(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 32) % n as u64) as usize
    }

    fn expr(&mut self, depth: usize) -> String {
        let branch = if depth == 0 { 0 } else { self.pick(9) };
        match branch {
            0 => ["0", "123u32", "0xFF", "true", "value", "'é'", "()"][self.pick(7)].into(),
            1 => format!("({} + {})", self.expr(depth - 1), self.expr(depth - 1)),
            2 => format!("f({}, {})", self.expr(depth - 1), self.expr(depth - 1)),
            3 => format!("[{}, {}]", self.expr(depth - 1), self.expr(depth - 1)),
            4 => format!(
                "if true {{ {} }} else {{ {} }}",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
            5 => format!("{{ let nested = {}; nested }}", self.expr(depth - 1)),
            6 => format!(
                "match value {{ 0 => {}, _ => {} }}",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
            7 => format!(
                "(|x: i64| {})({})",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
            _ => format!(
                "Pair {{ left: {}, right: {} }}",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
        }
    }

    fn program(&mut self, case: usize) -> String {
        let expr = self.expr(MAX_DEPTH);
        let literal = [
            r#""""#,
            r#""é λ 中 🦀 é""#,
            r#""\"\\\n\t\0\u{1F980}""#,
            r#""// not a comment: ([{}]) /// //!""#,
            r#"b"\x00\xFF\"\\""#,
            "\"first\nsecond\"",
        ][case % 6];
        let comment = ["// é 🦀 ([{}])", "//// plain, not docs", "// \\\" //!"][case % 3];
        let src = format!(
            "//! Generated é 🦀.\n\
             mod sample {{\n//! Nested docs.\n\
             /// Pair docs.\n#[derive(Copy)]\n\
             struct Pair {{ left: i64, right: i64 }}\n\
             enum Choice {{ A, B(i64) }}\n\
             trait Value {{ fn value(&self) -> i64; }}\n\
             impl Pair {{ fn value(&self) -> i64 {{ self.left }} }}\n\
             use crate::{{sample::Pair as Alias, sample::Choice}};\n\
             /// Function docs.\n#[allow(dead_code)]\n\
             pub fn generated_{case}() {{\n\
             {comment}\nlet value = {expr}; {comment}\n\
             let text = {literal};\nlet ch = '🦀';\n\
             f(value, // between arguments é\ntext);\n\
             }}\n}}\nfn main() {{}}\n// End 中\n"
        );
        assert!(src.len() <= MAX_BYTES);
        src
    }
}

/// Preserve the underlying assertion/panic and add enough context to
/// reproduce it with one filtered test, even when the panic is in production.
fn case<T>(label: &str, src: &str, check: impl FnOnce() -> T + UnwindSafe) -> T {
    assert!(src.len() <= MAX_BYTES, "{label}: input exceeds its bound");
    match catch_unwind(check) {
        Ok(value) => value,
        Err(panic) => {
            let why = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            panic!("{label}\nsource: {src:?}\n{why}");
        }
    }
}

fn span(src: &str, at: Span) {
    assert!(at.start <= at.end && at.end <= src.len(), "{at:?}");
    assert!(
        src.is_char_boundary(at.start) && src.is_char_boundary(at.end),
        "{at:?}"
    );
}

fn diagnostic(src: &str, error: &Error) {
    assert_eq!(error.phase, Phase::Syntax);
    span(src, error.span);
    for (at, _) in &error.notes {
        span(src, *at);
    }
    for fix in &error.fixes {
        for edit in &fix.edits {
            span(src, edit.span);
        }
        // Offering a fix must not panic on a UTF-8 boundary. It need not
        // make the whole source valid: there can be more than one error.
        let _ = fix.apply(src);
    }
    assert!(!error.render("generated.t", src).is_empty());
}

fn same_error(a: &Error, b: &Error) {
    assert_eq!(a.phase, b.phase);
    assert_eq!(a.span, b.span);
    assert_eq!(a.message, b.message);
    assert_eq!(a.notes, b.notes);
    assert_eq!(a.fixes, b.fixes);
}

fn module_references(src: &str, program: &Program) {
    for (i, module) in program.mods.iter().enumerate() {
        span(src, module.span);
        assert!(module.parent.map_or(i == ROOT, |parent| parent < i));
    }
    let modules = program
        .fns
        .iter()
        .map(|f| f.module)
        .chain(program.types.iter().map(|t| t.module()))
        .chain(program.traits.iter().map(|t| t.module))
        .chain(program.impls.iter().map(|i| i.module))
        .chain(program.uses.iter().map(|u| u.module));
    for module in modules {
        assert!(module < program.mods.len(), "dangling module {module}");
    }
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Valid,
    LexicalError,
    SyntaxError,
}

/// Strict parsing and formatting must agree on acceptance. Recovery must
/// agree on the first diagnostic, and on the complete AST for valid input.
fn check(src: &str) -> Outcome {
    let tokens = match lexer::lex(src) {
        Ok(tokens) => tokens,
        Err(error) => {
            diagnostic(src, &error);
            let Err(FmtError::Syntax(formatted)) = fmt::format(src) else {
                panic!("formatting must reject a lexical error");
            };
            same_error(&error, &formatted);
            return Outcome::LexicalError;
        }
    };
    for token in &tokens {
        span(src, token.span);
    }
    let editor = parse_for_editor(tokens.clone());
    assert!(
        editor.errors.len() <= tokens.len(),
        "recovery made no progress"
    );
    module_references(src, &editor.program);
    for error in &editor.errors {
        diagnostic(src, error);
    }
    match parse(tokens) {
        Err(error) => {
            diagnostic(src, &error);
            same_error(
                &error,
                editor
                    .errors
                    .first()
                    .expect("editor accepted invalid syntax"),
            );
            let Err(FmtError::Syntax(formatted)) = fmt::format(src) else {
                panic!("formatting must reject a syntax error");
            };
            same_error(&error, &formatted);
            Outcome::SyntaxError
        }
        Ok(program) => {
            assert!(editor.errors.is_empty(), "{:?}", editor.errors);
            assert_eq!(fmt::shape(&program), fmt::shape(&editor.program));
            let once = fmt::format(src).expect("valid source must format");
            let reparsed = parse(lexer::lex(&once).unwrap()).unwrap();
            assert_eq!(fmt::shape(&program), fmt::shape(&reparsed));
            assert_eq!(comments(src), comments(&once));
            assert_eq!(
                fmt::format(&once).unwrap(),
                once,
                "formatting isn't idempotent"
            );
            Outcome::Valid
        }
    }
}

fn comments(src: &str) -> Vec<String> {
    lexer::lex_with_comments(src)
        .unwrap()
        .1
        .into_iter()
        .map(|c| src[c.span.start..c.span.end].to_string())
        .collect()
}

#[test]
fn generated_valid_programs() {
    for seed in SEEDS {
        let mut generator = Generator(seed);
        for n in 0..PROGRAMS_PER_SEED {
            let src = generator.program(n);
            case(&format!("seed {seed:#x}, program {n}"), &src, || {
                assert_eq!(check(&src), Outcome::Valid);
            });
        }
    }
}

#[test]
fn generated_delimiter_mutations() {
    for seed in SEEDS {
        let mut generator = Generator(seed);
        for n in 0..PROGRAMS_PER_SEED {
            let src = generator.program(n);
            // Mutate token spans, so braces in strings/comments aren't
            // mistaken for syntax. Sample across the whole file, not just
            // its first header; include deletion and mismatched delimiters.
            let delimiters: Vec<_> = lexer::lex(&src)
                .unwrap()
                .into_iter()
                .filter(|t| {
                    matches!(
                        t.tok,
                        Tok::LParen
                            | Tok::RParen
                            | Tok::LBrace
                            | Tok::RBrace
                            | Tok::LBracket
                            | Tok::RBracket
                    )
                })
                .collect();
            let count = DELIMITERS_PER_PROGRAM.min(delimiters.len());
            for i in 0..count {
                let token = &delimiters[i * delimiters.len() / count];
                let wrong = match token.tok {
                    Tok::LParen | Tok::LBrace => "[",
                    Tok::RParen | Tok::RBrace => "]",
                    Tok::LBracket => "(",
                    _ => ")",
                };
                for replacement in ["", wrong] {
                    let mut mutated = src.clone();
                    mutated.replace_range(token.span.start..token.span.end, replacement);
                    case(
                        &format!("seed {seed:#x}, program {n}, delimiter {i} -> {replacement:?}"),
                        &mutated,
                        || check(&mutated),
                    );
                }
            }
            for close in [")", "]", "}"] {
                let mutated = format!("{src}\n{close}");
                case(
                    &format!("seed {seed:#x}, program {n}, extra {close}"),
                    &mutated,
                    || {
                        assert_eq!(check(&mutated), Outcome::SyntaxError);
                    },
                );
            }
        }
    }
}

#[test]
fn generated_truncations() {
    for seed in SEEDS {
        let mut generator = Generator(seed);
        // All character boundaries, including EOF, in four whole files.
        // Cutting through a token is intentional; cutting through UTF-8 isn't.
        for n in 0..2 {
            let src = generator.program(n + 2);
            for end in src.char_indices().map(|(i, _)| i).chain([src.len()]) {
                let prefix = &src[..end];
                case(
                    &format!("seed {seed:#x}, program {}, prefix {end}", n + 2),
                    prefix,
                    || check(prefix),
                );
            }
        }
    }
}

#[test]
fn generated_editor_recovery() {
    for seed in SEEDS {
        let mut generator = Generator(seed);
        for depth in 0..=MAX_DEPTH {
            let expr = generator.expr(MAX_DEPTH);
            for bad in [
                "fn broken(x: i64] { fn local() {} }".into(),
                format!("fn broken() {{ let x = ({expr}; fn local() {{}} }}"),
                "struct Broken { x: [i64; ] }".into(),
                "enum Broken { A(i64], B }".into(),
                "trait Broken { fn f(x:); }".into(),
                "impl Before { fn f(x:) {} }".into(),
                "#[allow(dead_code)] pub fn broken(x:) {}".into(),
                "use crate::{sample::,};".into(),
                "}".into(),
            ] {
                // A stray `}` closes a containing module, so it only has
                // an unambiguous surrounding-item oracle at the root.
                if depth > 0 && bad == "}" {
                    continue;
                }
                let mut src = "mod m { ".repeat(depth);
                src.push_str(&format!("struct Before {{}}\n{bad}\n/// Kept é 🦀.\n#[allow(dead_code)] pub fn after() {{}}\n"));
                src.push_str(&"} ".repeat(depth));
                src.push_str("fn main() {}");
                case(
                    &format!("seed {seed:#x}, recovery depth {depth}"),
                    &src,
                    || {
                        assert_eq!(check(&src), Outcome::SyntaxError);
                        let editor = parse_for_editor(lexer::lex(&src).unwrap());
                        // Recovery can report a leftover separator separately
                        // (the `;` after a malformed braced use tree). The
                        // contract is to keep surrounding complete items;
                        // check() already compares the first diagnostic.
                        assert_eq!(editor.program.mods.len(), depth + 1);
                        assert_eq!(editor.program.types.len(), 1);
                        assert_eq!(editor.program.types[0].name().name, "Before");
                        assert_eq!(editor.program.types[0].module(), depth);
                        let fns: Vec<_> = editor
                            .program
                            .fns
                            .iter()
                            .map(|f| (f.name.name.as_str(), f.module))
                            .collect();
                        assert_eq!(fns, [("after", depth), ("main", ROOT)]);
                        assert_eq!(
                            editor.program.fns[0].doc.as_ref().unwrap().markdown(),
                            "Kept é 🦀."
                        );
                        assert!(editor.program.traits.is_empty());
                        assert!(editor.program.impls.is_empty());
                        assert!(editor.program.uses.is_empty());
                    },
                );
            }
        }
    }

    for depth in 1..=MAX_DEPTH {
        let src = format!(
            "fn before() {{}} {}\n\
             use crate::Before; struct S {{}} enum E {{ A }}\n\
             trait T {{}} impl S {{}} fn child() {{}}\n\
             mod nested {{ struct Nested {{}} fn nested_child() {{}} }}",
            "mod unfinished { ".repeat(depth)
        );
        case(&format!("unclosed modules, depth {depth}"), &src, || {
            assert_eq!(check(&src), Outcome::SyntaxError);
            let editor = parse_for_editor(lexer::lex(&src).unwrap());
            assert_eq!(editor.program.mods.len(), 1);
            assert_eq!(editor.program.fns.len(), 1);
            assert_eq!(editor.program.fns[0].name.name, "before");
            assert!(editor.program.types.is_empty());
            assert!(editor.program.traits.is_empty());
            assert!(editor.program.impls.is_empty());
            assert!(editor.program.uses.is_empty());
        });
    }
}

#[test]
fn generated_quoted_inputs() {
    // Text and byte literals intentionally have different escape/Unicode
    // rules. Exercise every payload in strings and single-character forms,
    // with multibyte text before the diagnostic too.
    for (payload, text_ok, bytes_ok) in [
        ("", true, true),
        ("a", true, true),
        ("é", true, false),
        ("🦀", true, false),
        (r"\n", true, true),
        (r"\xFF", false, true),
        (r"\u{1F980}", true, false),
        (r"\u{10FFFF}", true, false),
        (r"\u{D800}", false, false),
        (r"\u{110000}", false, false),
        (r"\u{}", false, false),
        (r"\q", false, false),
        (r"\é", false, false),
        ("\\", false, false),
    ] {
        for prefix in ["", "b"] {
            for quote in ['"', '\''] {
                let src =
                    format!("// é 🦀\nfn main() {{ let x = {prefix}{quote}{payload}{quote}; }}");
                let valid = (if prefix.is_empty() { text_ok } else { bytes_ok })
                    && !(quote == '\'' && payload.is_empty());
                case("quoted literal matrix", &src, || {
                    assert_eq!(
                        check(&src),
                        if valid {
                            Outcome::Valid
                        } else {
                            Outcome::LexicalError
                        }
                    );
                });
            }
        }
    }
}

#[test]
fn generated_fragments() {
    let fragments = [
        "fn",
        "mod",
        "struct",
        "pub",
        "let",
        "x",
        "0",
        "0x",
        "255u8",
        "'a",
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        ";",
        ":",
        ",",
        "#",
        "=",
        "->",
        "<",
        ">",
        "\"",
        "\\",
        "\"é🦀\"",
        "'中'",
        "é",
        "🦀",
        "é",
        "\0",
        "\r\n",
        "// ([{}]) é\n",
        "/// Docs 🦀\n",
        "//! Inner 中\n",
        "//// plain\n",
        r#""\u{D800}""#,
        r#""\u{110000}""#,
        r#""\é""#,
        r#"b"\xF""#,
    ];
    let mut outcomes = [0; 3];
    for seed in SEEDS {
        let mut generator = Generator(seed);
        for n in 0..256 {
            let mut src = String::new();
            for _ in 0..generator.pick(48) {
                src.push_str(fragments[generator.pick(fragments.len())]);
                src.push_str(["", " ", "\n"][generator.pick(3)]);
            }
            let outcome = case(&format!("seed {seed:#x}, fragments {n}"), &src, || {
                check(&src)
            });
            outcomes[match outcome {
                Outcome::Valid => 0,
                Outcome::LexicalError => 1,
                Outcome::SyntaxError => 2,
            }] += 1;
        }
    }
    assert!(outcomes.into_iter().all(|count| count > 0), "{outcomes:?}");
}

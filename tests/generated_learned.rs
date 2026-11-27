//! Deterministic families for what a path learns of a value it didn't
//! make (docs/61-proof-search.md, lemma K1): a refinement of an array's
//! elements proved of `l`, a parameter, in one place, and asked of `l`
//! again in another. Learned by a `let` that declares it, an argument, an
//! assignment to a variable declared with it, a struct's field, or a
//! closure's body; in a branch whose condition makes it hold, one that
//! can't be taken, an arm of a `match` (one that matches a const, or a
//! pattern with alternatives, or with `..`), an iteration's branch, a
//! chain's block in an `if` or a `while`, or a closure; asked after the
//! branches meet, in the other branch, or later on the same path. Asked anywhere but later on the same path, it must be
//! proved again, and here it doesn't hold: refused. Later on the same
//! path, it's known: accepted, and run by the interpreter, which checks
//! each refinement as it's used. They're about what programs mean, so
//! they run on Linux alone.

use t_lang::error::Phase;

/// Checks `src` and, if it's accepted, runs it. Gives the error's message
/// if it's refused, or what went wrong running it.
fn verdict(src: &str) -> Result<(), String> {
    let checked = t_lang::check_source(src);
    let program = match checked.result {
        Ok(p) => p,
        Err(e) => {
            assert_eq!(
                e.phase,
                Phase::Type,
                "{src}\n{}",
                e.render_in(&checked.sources)
            );
            return Err(e.message);
        }
    };
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut terminal = t_lang::interp::Captured::default();
        t_lang::interp::run(&program.lower(), &mut terminal)
    }));
    match run {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!(
            "accepted, but it failed: {}",
            e.render_in(&checked.sources)
        )),
        Err(_) => Err("accepted, but the interpreter found a broken refinement".to_string()),
    }
}

/// How the refinement is proved of `l`: statements that prove it.
#[derive(Clone, Copy, Debug)]
enum Learn {
    Let,
    Argument,
    Assign,
    Field,
}

impl Learn {
    fn stmts(self) -> &'static str {
        match self {
            Learn::Let => "let y: Small = l;\n            let _ = y;",
            Learn::Argument => "let _ = take(n, l);",
            Learn::Assign => {
                "let mut y: Small = [0, 0];\n            y = l;\n            let _ = y;"
            }
            Learn::Field => "let h = Holder { n, a: l };\n            let _ = h;",
        }
    }
}

/// Where it's learned and where it's asked again.
#[derive(Clone, Copy, Debug)]
enum Place {
    /// In a branch whose condition makes it hold; asked after the `if`.
    AfterIf,
    /// In the `then` branch; asked in the `else`.
    OtherBranch,
    /// In an arm of a `match`; asked after it.
    AfterArm,
    /// In a branch that can't be taken; asked after it.
    DeadBranch,
    /// In a branch of a loop's body no iteration takes; asked in another.
    Iteration,
    /// In a branch of a closure's body, never called; asked after it's made.
    Closure,
    /// In a branch whose condition makes it hold; asked there, later.
    SamePath,
    /// In the block of a chain whose conditions make it hold; asked in its
    /// `else` (docs/100-patterns.md).
    ChainElse,
    /// The same; asked after the chain.
    AfterChain,
    /// The same; asked there, later.
    ChainSamePath,
    /// In the arm of a `match` that matches a const, `BIG`; asked after the
    /// `match`.
    AfterConstArm,
    /// The same; asked in the `_` arm.
    OtherArm,
    /// The same; asked there, later.
    ConstSamePath,
    /// In an arm whose pattern has alternatives, each matching `BIG`;
    /// asked after the `match`.
    AfterAlternatives,
    /// In an arm whose pattern has a `..`; asked after the `match`.
    AfterRestArm,
    /// In the block of a `while`'s chain; asked after the loop.
    AfterWhileChain,
    /// The same; asked there, later.
    WhileChainSamePath,
}

/// The function: `l`'s elements are below 10; `Small` says below `n`.
/// Learned where it holds (`n > 10`) or can't be reached, asked where it
/// doesn't: `f(3, [5, 5])`.
fn program(learn: Learn, place: Place) -> String {
    let s = learn.stmts();
    let ask = "let z: Small = l;\n    first(n, z)";
    let body = match place {
        Place::AfterIf => {
            format!("    if n > 10 {{\n        {{\n            {s}\n        }}\n    }}\n    {ask}")
        }
        Place::OtherBranch => format!(
            "    if n > 10 {{\n        {{\n            {s}\n        }}\n        0\n    }} else {{\n        {ask}\n    }}"
        ),
        Place::AfterArm => format!(
            "    match n {{\n        11 => {{\n            {s}\n        }}\n        _ => {{}}\n    }}\n    {ask}"
        ),
        Place::DeadBranch => format!(
            "    if n > 5 && n < 3 {{\n        {{\n            {s}\n        }}\n    }}\n    {ask}"
        ),
        Place::Iteration => format!(
            "    let mut r = 0u8;\n    for i in 0..3u8 {{\n        if i == 7 {{\n            {s}\n        }}\n        if i == 2 {{\n            let z: Small = l;\n            r = first(n, z);\n        }}\n    }}\n    r"
        ),
        Place::Closure => format!(
            "    let c = || {{\n        if n > 10 {{\n            {s}\n        }}\n    }};\n    let _ = c;\n    {ask}"
        ),
        Place::SamePath => format!(
            "    if n > 10 {{\n        {{\n            {s}\n        }}\n        let z: Small = l;\n        first(n, z)\n    }} else {{\n        0\n    }}"
        ),
        Place::ChainElse => format!(
            "    if let Some(_) = Some(1u8) && n > 10 {{\n        {{\n            {s}\n        }}\n        0\n    }} else {{\n        {ask}\n    }}"
        ),
        Place::AfterChain => format!(
            "    if let Some(_) = Some(1u8) && n > 10 {{\n        {{\n            {s}\n        }}\n    }}\n    {ask}"
        ),
        Place::ChainSamePath => format!(
            "    if let Some(_) = Some(1u8) && n > 10 {{\n        {{\n            {s}\n        }}\n        let z: Small = l;\n        first(n, z)\n    }} else {{\n        0\n    }}"
        ),
        Place::AfterConstArm => format!(
            "    match n {{\n        BIG => {{\n            {s}\n        }}\n        _ => {{}}\n    }}\n    {ask}"
        ),
        Place::OtherArm => format!(
            "    match n {{\n        BIG => {{\n            {{\n                {s}\n            }}\n            0\n        }}\n        _ => {{\n            {ask}\n        }}\n    }}"
        ),
        Place::ConstSamePath => format!(
            "    match n {{\n        BIG => {{\n            {{\n                {s}\n            }}\n            let z: Small = l;\n            first(n, z)\n        }}\n        _ => 0,\n    }}"
        ),
        Place::AfterAlternatives => format!(
            "    match (n, n) {{\n        (BIG, _) | (_, BIG) => {{\n            {s}\n        }}\n        _ => {{}}\n    }}\n    {ask}"
        ),
        Place::AfterRestArm => format!(
            "    match (n, 1u8, 2u8) {{\n        (BIG, ..) => {{\n            {s}\n        }}\n        _ => {{}}\n    }}\n    {ask}"
        ),
        Place::AfterWhileChain => format!(
            "    let mut go = true;\n    while go && let Some(_) = Some(1u8) && n > 10 {{\n        {{\n            {s}\n        }}\n        go = false;\n    }}\n    {ask}"
        ),
        Place::WhileChainSamePath => format!(
            "    let mut r = 0u8;\n    let mut go = true;\n    while go && let Some(_) = Some(1u8) && n > 10 {{\n        {{\n            {s}\n        }}\n        let z: Small = l;\n        r = first(n, z);\n        go = false;\n    }}\n    r"
        ),
    };
    let main_n = if same_path(place) { 11 } else { 3 };
    // `Small` is spelled out where it names `n`.
    let body = body.replace("Small", "[{ v: u8 | v < n }; 2]");
    format!(
        "const BIG: u8 = 11;

#[derive(Copy, Discard)]
struct Holder {{ n: u8, a: [{{ v: u8 | v < n }}; 2] }}

fn take(n: u8, y: [{{ v: u8 | v < n }}; 2]) -> u8 {{ y[0] }}

fn first(n: u8, z: [{{ v: u8 | v < n }}; 2]) -> {{ v: u8 | v < n }} {{ z[0] }}

fn f(n: u8, l: [{{ x: u8 | x < 10 }}; 2]) -> u8 {{
{body}
}}

fn main() {{
    let _ = f({main_n}, [5, 5]);
}}
"
    )
}

/// Whether `place` asks later on the path that learned it: then it's known.
fn same_path(place: Place) -> bool {
    matches!(
        place,
        Place::SamePath | Place::ChainSamePath | Place::ConstSamePath | Place::WhileChainSamePath
    )
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn learned_on_a_path_stays_on_it() {
    let learns = [Learn::Let, Learn::Argument, Learn::Assign, Learn::Field];
    let places = [
        Place::AfterIf,
        Place::OtherBranch,
        Place::AfterArm,
        Place::DeadBranch,
        Place::Iteration,
        Place::Closure,
        Place::SamePath,
        Place::ChainElse,
        Place::AfterChain,
        Place::ChainSamePath,
        Place::AfterConstArm,
        Place::OtherArm,
        Place::ConstSamePath,
        Place::AfterAlternatives,
        Place::AfterRestArm,
        Place::AfterWhileChain,
        Place::WhileChainSamePath,
    ];
    let mut wrong = Vec::new();
    let mut count = 0;
    for learn in learns {
        for place in places {
            let src = program(learn, place);
            count += 1;
            let got = verdict(&src);
            let ok = same_path(place);
            let right = match (&got, ok) {
                (Ok(()), true) => true,
                (Err(m), false) => m.contains("can't prove") && m.contains("for `z`"),
                _ => false,
            };
            if !right {
                wrong.push(format!(
                    "{learn:?} {place:?}: expected {}, got {got:?}\n{src}",
                    if ok { "accepted" } else { "refused" }
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {count} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

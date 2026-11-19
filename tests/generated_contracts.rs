//! Deterministic families for trait contracts
//! (docs/86-trait-contracts.md): a trait's method that says what it gives
//! back, what it asks of a parameter, what it leaves behind its `&mut self`
//! by a measure of the trait's, or what's in an `Option` it gives back;
//! impls of it for a struct, for a generic struct, through a reference to
//! another impl, by the trait's default body, or saying more than the
//! trait, each keeping the trait's word or breaking it; and callers that
//! count on it generically, of a known type, through another generic
//! function, and through a closure. Each is refused, or accepted and run by
//! the interpreter, which checks every impl's call against its trait's
//! contract as well as its own. They're about what programs mean, so they
//! run on Linux alone.

use t_lang::error::Phase;

/// Checks `src` and, if it's accepted, runs it: it must finish, having
/// broken no refinement. Gives the error's message if it's refused.
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
        Ok(Err(e)) => panic!("{src}\n{}", e.render_in(&checked.sources)),
        Err(_) => Err("the interpreter found a broken refinement".to_string()),
    }
}

/// What a trait's method promises or asks.
#[derive(Clone, Copy, PartialEq)]
enum Contract {
    /// `fn get(&self) -> { v: usize | v < 10 }`.
    Result,
    /// `fn get(&self, i: { i: usize | i < 4 }) -> usize`.
    Asks,
    /// `fn reset(&mut self) ensures self.size() == 0`, of a measure.
    Ensures,
    /// `fn get(&self) -> Option<{ v: usize | v < 10 }>`.
    Option,
}

/// How the trait is implemented.
#[derive(Clone, Copy, PartialEq)]
enum Impl {
    /// For a struct of its own.
    Own,
    /// For a generic struct, at any `X`.
    Generic,
    /// For a reference to a type that has its own impl, forwarding.
    Blanket,
    /// By the trait's default body, which the impl doesn't override.
    Default,
    /// Saying more than the trait does.
    Stronger,
}

/// The trait, and the method's body that keeps its word or doesn't.
fn trait_decl(c: Contract, body: Option<&str>) -> String {
    let end = |b: Option<&str>| match b {
        Some(b) => format!(" {{ {b} }}"),
        None => ";".to_string(),
    };
    match c {
        Contract::Result => format!(
            "trait Tr {{\n    fn n(&self) -> usize;\n\n    fn get(&self) -> {{ v: usize | v < 10 }}{}\n}}\n",
            end(body)
        ),
        Contract::Asks => format!(
            "trait Tr {{\n    fn n(&self) -> usize;\n\n    fn get(&self, i: {{ i: usize | i < 4 }}) -> usize{}\n}}\n",
            end(body)
        ),
        Contract::Ensures => format!(
            "trait Tr {{\n    #[measure]\n    fn size(&self) -> usize;\n\n    fn n(&self) -> usize;\n\n    \
             fn reset(&mut self) ensures self.size() == 0{}\n}}\n",
            end(body)
        ),
        Contract::Option => format!(
            "trait Tr {{\n    fn n(&self) -> usize;\n\n    fn get(&self) -> Option<{{ v: usize | v < 10 }}>{}\n}}\n",
            end(body)
        ),
    }
}

/// The method's body, in an impl of `S`, whose `n` may be anything, or in
/// the trait's default, of `self.n()`: keeping the trait's word, or not.
fn body(c: Contract, honest: bool, default: bool) -> &'static str {
    match (c, honest, default) {
        (Contract::Result, true, false) => "if self.n < 10 { self.n } else { 0 }",
        (Contract::Result, false, false) => "self.n",
        (Contract::Result, true, true) => "let n = self.n(); if n < 10 { n } else { 0 }",
        (Contract::Result, false, true) => "self.n()",
        // An impl's body may count on what the trait asks.
        (Contract::Asks, true, false) => "let a = [1usize, 2, 3, 4]; a[i]",
        (Contract::Asks, false, false) => "let a = [1usize, 2, 3]; a[i]",
        (Contract::Asks, true, true) => "let a = [1usize, 2, 3, 4]; a[i]",
        (Contract::Asks, false, true) => "let a = [1usize, 2, 3]; a[i]",
        (Contract::Ensures, true, false) => "self.n = 0;",
        (Contract::Ensures, false, false) => "self.n = 1;",
        (Contract::Ensures, _, true) => unreachable!("no default: its measure is the impl's"),
        (Contract::Option, true, false) => "if self.n < 10 { Some(self.n) } else { None }",
        (Contract::Option, false, false) => "Some(self.n)",
        (Contract::Option, true, true) => "let n = self.n(); if n < 10 { Some(n) } else { None }",
        (Contract::Option, false, true) => "Some(self.n())",
    }
}

/// The method's signature as an impl writes it, saying no more.
fn plain_sig(c: Contract) -> &'static str {
    match c {
        Contract::Result => "fn get(&self) -> usize",
        Contract::Asks => "fn get(&self, i: usize) -> usize",
        Contract::Ensures => "fn reset(&mut self)",
        Contract::Option => "fn get(&self) -> Option<usize>",
    }
}

/// The method's signature, saying more than the trait's.
fn stronger_sig(c: Contract) -> &'static str {
    match c {
        Contract::Result => "fn get(&self) -> { v: usize | v < 20 }",
        Contract::Asks => "fn get(&self, i: { i: usize | i < 5 }) -> usize",
        Contract::Ensures => "fn reset(&mut self) ensures self.size() <= 1",
        Contract::Option => "fn get(&self) -> Option<{ v: usize | v < 20 }>",
    }
}

/// The impls, of `S { n }` and maybe more.
fn impls(c: Contract, i: Impl, honest: bool) -> String {
    let measure = match c {
        Contract::Ensures => "    #[measure]\n    fn size(&self) -> usize { self.n }\n\n",
        _ => "",
    };
    let n = "    fn n(&self) -> usize { self.n }\n\n";
    let method = |sig: &str, b: &str| format!("    {sig} {{ {b} }}\n");
    let own = |sig: &str| {
        format!(
            "impl Tr for S {{\n{measure}{n}{}}}\n",
            method(sig, body(c, honest, false))
        )
    };
    match i {
        Impl::Own => own(plain_sig(c)),
        // Asking less than the trait, `i < 5`, it must handle `4` too.
        Impl::Stronger if c == Contract::Asks => format!(
            "impl Tr for S {{\n{n}{}}}\n",
            method(
                stronger_sig(c),
                match honest {
                    true => "let a = [1usize, 2, 3, 4, 5]; a[i]",
                    false => "let a = [1usize, 2, 3, 4]; a[i]",
                }
            )
        ),
        Impl::Stronger => own(stronger_sig(c)),
        Impl::Default => format!("impl Tr for S {{\n{measure}{}}}\n", n.trim_end()),
        Impl::Generic => format!(
            "impl<X: Discard> Tr for W<X> {{\n{measure}{n}{}}}\n",
            method(plain_sig(c), body(c, honest, false))
        ),
        Impl::Blanket => {
            // `S`'s keeps its word or not; the reference's forwards it.
            let (forward_sig, forward) = match c {
                Contract::Result | Contract::Option => (plain_sig(c), "(**self).get()"),
                Contract::Asks => (plain_sig(c), "(**self).get(i)"),
                Contract::Ensures => unreachable!("a measure can't read through a `&mut`"),
            };
            format!(
                "{}\nimpl<'a, T: Tr> Tr for &'a T {{\n    fn n(&self) -> usize {{ (**self).n() }}\n\n{}}}\n",
                own(plain_sig(c)),
                method(forward_sig, forward)
            )
        }
    }
}

/// How a caller counts on the contract.
#[derive(Clone, Copy, PartialEq)]
enum Caller {
    /// Generic in the trait.
    Generic,
    /// Of a type it knows.
    Known,
    /// Generic, through another generic function.
    Nested,
    /// Through a closure.
    Closure,
    /// Generic, asking what the trait doesn't allow (only for `Asks`).
    Breaks,
}

/// A generic function counting on the contract: indexing an array of 10
/// with what it gives, or giving `i` of 3, or after `reset`, indexing an
/// array of 1 with `size`.
fn use_it(c: Contract, breaks: bool) -> String {
    let (head, body) = match c {
        Contract::Result => ("t: &T", "let a = [7u8; 10];\n    a[t.get()]".to_string()),
        Contract::Asks => (
            "t: &T",
            format!(
                "let a = [7u8; 10];\n    a[t.get({}) % 10]",
                if breaks { 4 } else { 3 }
            ),
        ),
        Contract::Ensures => (
            "t: &mut T",
            "t.reset();\n    let a = [7u8; 1];\n    a[t.size()]".to_string(),
        ),
        Contract::Option => (
            "t: &T",
            "let a = [7u8; 10];\n    match t.get() {\n        Some(v) => a[v],\n        None => 0,\n    }"
                .to_string(),
        ),
    };
    format!("fn use_it<T: Tr>({head}) -> u8 {{\n    {body}\n}}\n")
}

/// The whole program: `main` makes a value of the impl's type with `n` of
/// 10, which a lying body gives away, and calls as `caller` does.
fn program(c: Contract, i: Impl, honest: bool, caller: Caller) -> String {
    let mut src = String::new();
    let default = match (i, c) {
        (Impl::Default, _) => Some(body(c, honest, true)),
        _ => None,
    };
    src.push_str(&trait_decl(c, default));
    src.push_str("\n#[derive(Discard)]\nstruct S { n: usize }\n\n");
    src.push_str("#[derive(Discard)]\nstruct W<X> { x: X, n: usize }\n\n");
    src.push_str(&impls(c, i, honest));
    src.push('\n');
    src.push_str(&use_it(c, caller == Caller::Breaks));
    src.push_str(
        "\nfn nested<T: Tr>(t: TT) -> u8 { use_it(t) }\n"
            .replace(
                "TT",
                match c {
                    Contract::Ensures => "&mut T",
                    _ => "&T",
                },
            )
            .as_str(),
    );
    let (ty, make) = match i {
        Impl::Generic => ("W<u8>", "W { x: 1u8, n: 10 }"),
        _ => ("S", "S { n: 10 }"),
    };
    let (mut_, amp) = match c {
        Contract::Ensures => ("mut ", "&mut "),
        _ => ("", "&"),
    };
    // Through the blanket impl, the reference is what implements it.
    let arg = match i {
        Impl::Blanket => format!("&{amp}v"),
        _ => format!("{amp}v"),
    };
    let call = match caller {
        Caller::Generic | Caller::Breaks => format!("use_it({arg})"),
        Caller::Nested => format!("nested({arg})"),
        Caller::Closure => format!("{{ let f = |w: {amp}{ty}| use_it(w); f({amp}v) }}"),
        Caller::Known => match c {
            Contract::Result => "{ let a = [7u8; 10]; a[v.get()] }".to_string(),
            Contract::Asks => "{ let a = [7u8; 10]; a[v.get(3) % 10] }".to_string(),
            Contract::Ensures => "{ v.reset(); let a = [7u8; 1]; a[v.size()] }".to_string(),
            Contract::Option => {
                "{ let a = [7u8; 10]; match v.get() { Some(k) => a[k], None => 0 } }".to_string()
            }
        },
    };
    src.push_str(&format!(
        "\nfn main() -> Result<(), u8> {{\n    let {mut_}v: {ty} = {make};\n    let _ = {call};\n    Ok(())\n}}\n"
    ));
    src
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn contracts() {
    let contracts = [
        Contract::Result,
        Contract::Asks,
        Contract::Ensures,
        Contract::Option,
    ];
    let impls_ = [
        Impl::Own,
        Impl::Generic,
        Impl::Blanket,
        Impl::Default,
        Impl::Stronger,
    ];
    let callers = [
        Caller::Generic,
        Caller::Known,
        Caller::Nested,
        Caller::Closure,
        Caller::Breaks,
    ];
    let mut failures = Vec::new();
    let mut count = 0;
    for c in contracts {
        for i in impls_ {
            // A measure can't read through the reference's `&mut`, nor be
            // a default of a field the trait can't see.
            if c == Contract::Ensures && matches!(i, Impl::Blanket | Impl::Default) {
                continue;
            }
            for honest in [true, false] {
                for caller in callers {
                    if caller == Caller::Breaks && c != Contract::Asks {
                        continue;
                    }
                    // Of a known type, through the blanket impl, is of `S`'s.
                    if caller == Caller::Known && i == Impl::Blanket {
                        continue;
                    }
                    count += 1;
                    let src = program(c, i, honest, caller);
                    let what = format!(
                        "{} by {} that {}, called {}",
                        match c {
                            Contract::Result => "result",
                            Contract::Asks => "asks",
                            Contract::Ensures => "ensures",
                            Contract::Option => "option",
                        },
                        match i {
                            Impl::Own => "its own impl",
                            Impl::Generic => "a generic impl",
                            Impl::Blanket => "a blanket impl",
                            Impl::Default => "the default",
                            Impl::Stronger => "a stronger impl",
                        },
                        if honest { "keeps its word" } else { "lies" },
                        match caller {
                            Caller::Generic => "generically",
                            Caller::Known => "knowing the type",
                            Caller::Nested => "through a generic function",
                            Caller::Closure => "through a closure",
                            Caller::Breaks => "asking too much",
                        }
                    );
                    // The impl's own refinements, saying more, are what a
                    // lying body breaks first, or what a stronger `Asks`
                    // impl can't have.
                    let at_impl = |m: &str| {
                        m.contains("of `get`")
                            || m.contains("`reset` ensures")
                            || m.contains("argument `i` of `index`")
                            || m.contains("from what trait `Tr` asks")
                    };
                    match (verdict(&src), honest, caller) {
                        (Err(m), _, Caller::Breaks) if m.contains("argument `i` of `get`") => {}
                        (Ok(()), true, c) if c != Caller::Breaks => {}
                        (Err(m), false, _) if at_impl(&m) => {}
                        (Err(m), _, _) => failures.push(format!("{what}: refused: {m}\n{src}")),
                        (Ok(()), _, _) => failures.push(format!("{what}: accepted\n{src}")),
                    }
                }
            }
        }
    }
    assert!(count > 100, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

// Functions that call each other get their effects together: the least
// that holds for all of them. Here an expression parser over bytes, whose
// functions recurse through each other, and only `atom` allocates.
use alloc::boxed::Box;
use core::num::{wrapping_add, wrapping_sub};

#[derive(Discard)]
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

/// The byte at `at`, or 0 past the end.
fn byte(s: &[u8], at: usize) -> u8 {
    match s.get(at) {
        Option::Some(c) => *c,
        Option::None => 0,
    }
}

fn next(at: usize) -> usize { wrapping_add(at, 1) }

fn expr(s: &[u8], at: usize) -> Pair {
    let Pair { e: left, at } = unary(s, at);
    if byte(s, at) == b'+' {
        let Pair { e: right, at } = expr(s, next(at));
        Pair { e: Expr::Add(Box::new(left), Box::new(right)), at }
    } else {
        Pair { e: left, at }
    }
}

fn unary(s: &[u8], at: usize) -> Pair {
    if byte(s, at) == b'-' {
        let Pair { e, at } = unary(s, next(at));
        Pair { e: Expr::Neg(Box::new(e)), at }
    } else {
        atom(s, at)
    }
}

fn atom(s: &[u8], at: usize) -> Pair {
    let c = byte(s, at);
    if c == b'(' {
        let Pair { e, at } = expr(s, next(at));
        Pair { e, at: next(at) }
    } else {
        Pair { e: Expr::Num(wrapping_sub(c, b'0') as i64), at: next(at) }
    }
}

#[derive(Discard)]
struct Pair { e: Expr, at: usize }

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
        Expr::Neg(a) => -eval(a),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let Pair { e, .. } = expr(b"1+-(2+3)+4", 0);
    out.line(&eval(&e))?; // expect: 0
    Result::Ok(())
}

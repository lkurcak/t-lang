//! Turns source text into a flat list of tokens.

use crate::error::{Edit, Error, Fix, Span};
use crate::types::IntTy;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// An integer literal: its value, and the type its suffix says, as
    /// in `255u8`, if it has one.
    Int(u64, Option<IntTy>),
    Ident(String),
    // keywords
    Fn,
    Let,
    If,
    Else,
    True,
    False,
    Struct,
    Enum,
    Trait,
    Impl,
    For,
    In,
    Match,
    Mut,
    Move,
    While,
    Loop,
    Unsafe,
    Return,
    Break,
    Continue,
    Type,
    As,
    Pub,
    Mod,
    Use,
    Crate,
    Super,
    Where,
    With,
    Effect,
    /// `'a`: a lifetime, with its `'`.
    Lifetime(String),
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Hash,
    Comma,
    Colon,
    ColonColon,
    Semi,
    Dot,
    DotDot,
    DotDotEq,
    Arrow,
    FatArrow,
    // operators
    Eq,
    /// `+=`, `-=` and the others: an operator and `=`, assigning.
    OpEq(OpEq),
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Amp,
    AndAnd,
    OrOr,
    Pipe,
    Caret,
    /// `<<`. There's no `>>` token: a type's arguments may end with two
    /// `>`s, as in `Option<Option<u8>>`, so the parser takes two `>`s
    /// with nothing between them as a shift.
    Shl,
    /// A `///` doc comment line: the text after the `///`. The parser sets
    /// these aside, and attaches them to what comes after.
    Doc(String),
    Eof,
}

impl Tok {
    /// How the token is shown in error messages.
    pub fn describe(&self) -> String {
        let s = match self {
            Tok::Int(n, None) => return format!("`{n}`"),
            Tok::Int(n, Some(t)) => return format!("`{n}{}`", t.name()),
            Tok::Ident(name) => return format!("`{name}`"),
            Tok::Doc(_) => return "a doc comment".to_string(),
            Tok::Lifetime(name) => return format!("`{name}`"),
            Tok::OpEq(op) => return format!("`{}=`", op.symbol()),
            Tok::Eof => return "end of file".to_string(),
            Tok::Fn => "fn",
            Tok::Let => "let",
            Tok::If => "if",
            Tok::Else => "else",
            Tok::True => "true",
            Tok::False => "false",
            Tok::Struct => "struct",
            Tok::Enum => "enum",
            Tok::Trait => "trait",
            Tok::Impl => "impl",
            Tok::For => "for",
            Tok::In => "in",
            Tok::Match => "match",
            Tok::Mut => "mut",
            Tok::Move => "move",
            Tok::While => "while",
            Tok::Loop => "loop",
            Tok::Unsafe => "unsafe",
            Tok::Return => "return",
            Tok::Break => "break",
            Tok::Continue => "continue",
            Tok::Type => "type",
            Tok::As => "as",
            Tok::Pub => "pub",
            Tok::Mod => "mod",
            Tok::Use => "use",
            Tok::Crate => "crate",
            Tok::Super => "super",
            Tok::Where => "where",
            Tok::With => "with",
            Tok::Effect => "effect",
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::LBrace => "{",
            Tok::RBrace => "}",
            Tok::LBracket => "[",
            Tok::RBracket => "]",
            Tok::Hash => "#",
            Tok::Comma => ",",
            Tok::Colon => ":",
            Tok::ColonColon => "::",
            Tok::Semi => ";",
            Tok::Dot => ".",
            Tok::DotDot => "..",
            Tok::DotDotEq => "..=",
            Tok::Arrow => "->",
            Tok::FatArrow => "=>",
            Tok::Eq => "=",
            Tok::EqEq => "==",
            Tok::NotEq => "!=",
            Tok::Lt => "<",
            Tok::Le => "<=",
            Tok::Gt => ">",
            Tok::Ge => ">=",
            Tok::Plus => "+",
            Tok::Minus => "-",
            Tok::Star => "*",
            Tok::Slash => "/",
            Tok::Percent => "%",
            Tok::Bang => "!",
            Tok::Amp => "&",
            Tok::AndAnd => "&&",
            Tok::OrOr => "||",
            Tok::Pipe => "|",
            Tok::Caret => "^",
            Tok::Shl => "<<",
        };
        format!("`{s}`")
    }
}

/// The operator of a compound assignment like `+=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpEq {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

impl OpEq {
    pub fn symbol(self) -> &'static str {
        match self {
            OpEq::Add => "+",
            OpEq::Sub => "-",
            OpEq::Mul => "*",
            OpEq::Div => "/",
            OpEq::Rem => "%",
            OpEq::And => "&",
            OpEq::Or => "|",
            OpEq::Xor => "^",
            OpEq::Shl => "<<",
            OpEq::Shr => ">>",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

/// A `// ...` comment. The parser never sees these; `t fmt` keeps them.
/// A `///` doc comment is a token instead (`Tok::Doc`).
#[derive(Debug, Clone)]
pub struct Comment {
    pub span: Span,
}

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    Ok(lex_with_comments(src)?.0)
}

/// The tokens, and the comments between them, both in order.
pub fn lex_with_comments(src: &str) -> Result<(Vec<Token>, Vec<Comment>), Error> {
    let bytes = src.as_bytes();
    let mut toks = Vec::new();
    let mut comments = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i];
        let start = i;

        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            let end = start + src[start..i].trim_end().len();
            let span = Span::new(start, end);
            // `///`, but not `////`, which is a plain comment, as in Rust.
            match src[start..end].strip_prefix("///") {
                Some(text) if !text.starts_with('/') => toks.push(Token {
                    tok: Tok::Doc(text.to_string()),
                    span,
                }),
                _ => comments.push(Comment { span }),
            }
            continue;
        }

        let tok = if c.is_ascii_digit() {
            let radix = match (c, bytes.get(i + 1)) {
                (b'0', Some(b'x')) => 16,
                (b'0', Some(b'b')) => 2,
                (b'0', Some(b'o')) => 8,
                (b'0', Some(&p @ (b'X' | b'B' | b'O'))) => {
                    let lower = (p as char).to_ascii_lowercase();
                    let span = Span::new(start, start + 2);
                    return Err(
                        Error::syntax(span, format!("write `0{lower}`, in lowercase")).with_fix(
                            Fix::new(
                                format!("write `0{lower}`"),
                                vec![Edit::replace(span, format!("0{lower}"))],
                            ),
                        ),
                    );
                }
                _ => 10,
            };
            if radix != 10 {
                i += 2;
            }
            // Its digits: hexadecimal ones for any radix, so that `0b102`
            // says `2` isn't a binary digit rather than that `2` isn't a
            // suffix. A suffix starts with `i` or `u`, which aren't any.
            let at = i;
            while i < bytes.len() && (bytes[i].is_ascii_hexdigit() || bytes[i] == b'_') {
                i += 1;
            }
            let digits: String = src[at..i].chars().filter(|&c| c != '_').collect();
            if digits.is_empty() {
                let prefix = &src[start..at];
                return Err(Error::syntax(
                    Span::new(start, at),
                    format!("`{prefix}` needs digits after it"),
                ));
            }
            if let Some(off) = src[at..i].find(|c: char| !c.is_digit(radix) && c != '_') {
                let d = &src[at + off..at + off + 1];
                let what = match radix {
                    2 => "a binary digit",
                    8 => "an octal digit",
                    _ => "a decimal digit",
                };
                return Err(Error::syntax(
                    Span::new(at + off, at + off + 1),
                    format!("`{d}` isn't {what}"),
                ));
            }
            let n = u64::from_str_radix(&digits, radix)
                .map_err(|_| Error::syntax(Span::new(start, i), "integer literal is too large"))?;
            // A suffix, `255u8`, says which integer type it is.
            let at = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let suffix = match &src[at..i] {
                "" => None,
                s => Some(IntTy::named(s).ok_or_else(|| {
                    Error::syntax(
                        Span::new(at, i),
                        format!(
                            "`{s}` isn't an integer type, so it can't be a literal's suffix, as \
                             `u8` is in `255u8`"
                        ),
                    )
                })?),
            };
            Tok::Int(n, suffix)
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            match &src[start..i] {
                "fn" => Tok::Fn,
                "let" => Tok::Let,
                "if" => Tok::If,
                "else" => Tok::Else,
                "true" => Tok::True,
                "false" => Tok::False,
                "struct" => Tok::Struct,
                "enum" => Tok::Enum,
                "trait" => Tok::Trait,
                "impl" => Tok::Impl,
                "for" => Tok::For,
                "in" => Tok::In,
                "match" => Tok::Match,
                "mut" => Tok::Mut,
                "move" => Tok::Move,
                "while" => Tok::While,
                "loop" => Tok::Loop,
                "unsafe" => Tok::Unsafe,
                "return" => Tok::Return,
                "break" => Tok::Break,
                "continue" => Tok::Continue,
                "type" => Tok::Type,
                "as" => Tok::As,
                "pub" => Tok::Pub,
                "mod" => Tok::Mod,
                "use" => Tok::Use,
                "crate" => Tok::Crate,
                "super" => Tok::Super,
                "where" => Tok::Where,
                "with" => Tok::With,
                "effect" => Tok::Effect,
                name => Tok::Ident(name.to_string()),
            }
        } else if c == b'\'' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if i == start + 1 || bytes[start + 1].is_ascii_digit() {
                let span = Span::new(start, start + 1);
                return Err(Error::syntax(
                    span,
                    "expected a lifetime name after `'`, like `'a`",
                ));
            }
            Tok::Lifetime(src[start..i].to_string())
        } else {
            let next = bytes.get(i + 1).copied();
            let third = bytes.get(i + 2).copied();
            let (tok, len) = match (c, next) {
                (b'.', Some(b'.')) if third == Some(b'=') => (Tok::DotDotEq, 3),
                (b'.', Some(b'.')) => (Tok::DotDot, 2),
                (b'-', Some(b'>')) => (Tok::Arrow, 2),
                (b'=', Some(b'>')) => (Tok::FatArrow, 2),
                (b':', Some(b':')) => (Tok::ColonColon, 2),
                (b'=', Some(b'=')) => (Tok::EqEq, 2),
                (b'!', Some(b'=')) => (Tok::NotEq, 2),
                (b'<', Some(b'=')) => (Tok::Le, 2),
                (b'>', Some(b'=')) => (Tok::Ge, 2),
                (b'&', Some(b'&')) => (Tok::AndAnd, 2),
                (b'|', Some(b'|')) => (Tok::OrOr, 2),
                (b'+', Some(b'=')) => (Tok::OpEq(OpEq::Add), 2),
                (b'-', Some(b'=')) => (Tok::OpEq(OpEq::Sub), 2),
                (b'*', Some(b'=')) => (Tok::OpEq(OpEq::Mul), 2),
                (b'/', Some(b'=')) => (Tok::OpEq(OpEq::Div), 2),
                (b'%', Some(b'=')) => (Tok::OpEq(OpEq::Rem), 2),
                (b'&', Some(b'=')) => (Tok::OpEq(OpEq::And), 2),
                (b'|', Some(b'=')) => (Tok::OpEq(OpEq::Or), 2),
                (b'^', Some(b'=')) => (Tok::OpEq(OpEq::Xor), 2),
                (b'<', Some(b'<')) if third == Some(b'=') => (Tok::OpEq(OpEq::Shl), 3),
                (b'<', Some(b'<')) => (Tok::Shl, 2),
                (b'^', _) => (Tok::Caret, 1),
                (b'&', _) => (Tok::Amp, 1),
                (b'|', _) => (Tok::Pipe, 1),
                (b'(', _) => (Tok::LParen, 1),
                (b')', _) => (Tok::RParen, 1),
                (b'{', _) => (Tok::LBrace, 1),
                (b'}', _) => (Tok::RBrace, 1),
                (b'[', _) => (Tok::LBracket, 1),
                (b']', _) => (Tok::RBracket, 1),
                (b'#', _) => (Tok::Hash, 1),
                (b',', _) => (Tok::Comma, 1),
                (b':', _) => (Tok::Colon, 1),
                (b';', _) => (Tok::Semi, 1),
                (b'.', _) => (Tok::Dot, 1),
                (b'=', _) => (Tok::Eq, 1),
                (b'<', _) => (Tok::Lt, 1),
                (b'>', _) => (Tok::Gt, 1),
                (b'+', _) => (Tok::Plus, 1),
                (b'-', _) => (Tok::Minus, 1),
                (b'*', _) => (Tok::Star, 1),
                (b'/', _) => (Tok::Slash, 1),
                (b'%', _) => (Tok::Percent, 1),
                (b'!', _) => (Tok::Bang, 1),
                _ => {
                    let ch = src[start..].chars().next().unwrap();
                    let span = Span::new(start, start + ch.len_utf8());
                    return Err(Error::syntax(span, format!("unexpected character `{ch}`")));
                }
            };
            i += len;
            tok
        };
        toks.push(Token {
            tok,
            span: Span::new(start, i),
        });
    }

    toks.push(Token {
        tok: Tok::Eof,
        span: Span::new(src.len(), src.len()),
    });
    Ok((toks, comments))
}

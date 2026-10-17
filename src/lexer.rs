//! Turns source text into a flat list of tokens.

use crate::error::{Error, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Int(i64),
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
    Match,
    Mut,
    While,
    Loop,
    Return,
    Break,
    Continue,
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
    /// A `///` doc comment line: the text after the `///`. The parser sets
    /// these aside, and attaches them to what comes after.
    Doc(String),
    Eof,
}

impl Tok {
    /// How the token is shown in error messages.
    pub fn describe(&self) -> String {
        let s = match self {
            Tok::Int(n) => return format!("`{n}`"),
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
            Tok::Match => "match",
            Tok::Mut => "mut",
            Tok::While => "while",
            Tok::Loop => "loop",
            Tok::Return => "return",
            Tok::Break => "break",
            Tok::Continue => "continue",
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
}

impl OpEq {
    pub fn symbol(self) -> &'static str {
        match self {
            OpEq::Add => "+",
            OpEq::Sub => "-",
            OpEq::Mul => "*",
            OpEq::Div => "/",
            OpEq::Rem => "%",
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
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
            let digits: String = src[start..i].chars().filter(|&c| c != '_').collect();
            let n = digits
                .parse::<i64>()
                .map_err(|_| Error::syntax(Span::new(start, i), "integer literal is too large"))?;
            Tok::Int(n)
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
                "match" => Tok::Match,
                "mut" => Tok::Mut,
                "while" => Tok::While,
                "loop" => Tok::Loop,
                "return" => Tok::Return,
                "break" => Tok::Break,
                "continue" => Tok::Continue,
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

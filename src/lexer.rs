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
    Match,
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    ColonColon,
    Semi,
    Dot,
    Arrow,
    FatArrow,
    // operators
    Eq,
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
    AndAnd,
    OrOr,
    Pipe,
    Eof,
}

impl Tok {
    /// How the token is shown in error messages.
    pub fn describe(&self) -> String {
        let s = match self {
            Tok::Int(n) => return format!("`{n}`"),
            Tok::Ident(name) => return format!("`{name}`"),
            Tok::Eof => return "end of file".to_string(),
            Tok::Fn => "fn",
            Tok::Let => "let",
            Tok::If => "if",
            Tok::Else => "else",
            Tok::True => "true",
            Tok::False => "false",
            Tok::Struct => "struct",
            Tok::Enum => "enum",
            Tok::Match => "match",
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::LBrace => "{",
            Tok::RBrace => "}",
            Tok::Comma => ",",
            Tok::Colon => ":",
            Tok::ColonColon => "::",
            Tok::Semi => ";",
            Tok::Dot => ".",
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
            Tok::AndAnd => "&&",
            Tok::OrOr => "||",
            Tok::Pipe => "|",
        };
        format!("`{s}`")
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let bytes = src.as_bytes();
    let mut toks = Vec::new();
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
                "match" => Tok::Match,
                name => Tok::Ident(name.to_string()),
            }
        } else {
            let next = bytes.get(i + 1).copied();
            let (tok, len) = match (c, next) {
                (b'-', Some(b'>')) => (Tok::Arrow, 2),
                (b'=', Some(b'>')) => (Tok::FatArrow, 2),
                (b':', Some(b':')) => (Tok::ColonColon, 2),
                (b'=', Some(b'=')) => (Tok::EqEq, 2),
                (b'!', Some(b'=')) => (Tok::NotEq, 2),
                (b'<', Some(b'=')) => (Tok::Le, 2),
                (b'>', Some(b'=')) => (Tok::Ge, 2),
                (b'&', Some(b'&')) => (Tok::AndAnd, 2),
                (b'|', Some(b'|')) => (Tok::OrOr, 2),
                (b'|', _) => (Tok::Pipe, 1),
                (b'(', _) => (Tok::LParen, 1),
                (b')', _) => (Tok::RParen, 1),
                (b'{', _) => (Tok::LBrace, 1),
                (b'}', _) => (Tok::RBrace, 1),
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
    Ok(toks)
}

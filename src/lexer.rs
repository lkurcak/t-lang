//! Turns source text into a flat list of tokens.

use crate::error::{Edit, Error, Fix, Span};
use crate::types::IntTy;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// An integer literal: its value, and the type its suffix says, as
    /// in `255u8`, if it has one.
    Int(u64, Option<IntTy>),
    /// `b'a'`: a byte literal, a `u8`.
    Byte(u8),
    /// `'a'`: a character literal.
    Char(char),
    /// `"hi"`: a string literal, its escapes worked out.
    Str(String),
    /// `b"hi"`: a byte string literal, its escapes worked out.
    ByteStr(Vec<u8>),
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
    Export,
    Mod,
    Use,
    Crate,
    Super,
    Where,
    Ensures,
    With,
    Effect,
    Sealed,
    Const,
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
    /// `?`, after an expression: return early (step 15).
    Question,
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
    /// A `//!` doc comment line, documenting the module it starts: the
    /// text after the `//!`.
    InnerDoc(String),
    Eof,
}

impl Tok {
    /// How the token is shown in error messages.
    pub fn describe(&self) -> String {
        let s = match self {
            Tok::Int(n, None) => return format!("`{n}`"),
            Tok::Int(n, Some(t)) => return format!("`{n}{}`", t.name()),
            Tok::Byte(b) => return format!("`{}`", byte_literal(*b)),
            Tok::Char(c) => return format!("`{}`", char_literal(*c)),
            Tok::Str(_) => return "a string literal".to_string(),
            Tok::ByteStr(_) => return "a byte string literal".to_string(),
            Tok::Ident(name) => return format!("`{name}`"),
            Tok::Doc(_) | Tok::InnerDoc(_) => return "a doc comment".to_string(),
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
            Tok::Export => "export",
            Tok::Mod => "mod",
            Tok::Use => "use",
            Tok::Crate => "crate",
            Tok::Super => "super",
            Tok::Where => "where",
            Tok::Ensures => "ensures",
            Tok::With => "with",
            Tok::Effect => "effect",
            Tok::Sealed => "sealed",
            Tok::Const => "const",
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::LBrace => "{",
            Tok::RBrace => "}",
            Tok::LBracket => "[",
            Tok::RBracket => "]",
            Tok::Hash => "#",
            Tok::Question => "?",
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

/// Which kind of quoted literal `quoted` reads.
#[derive(Clone, Copy, PartialEq)]
enum Quote {
    Str,
    Char,
    ByteStr,
    Byte,
}

impl Quote {
    fn bytes(self) -> bool {
        matches!(self, Quote::ByteStr | Quote::Byte)
    }

    fn one(self) -> bool {
        matches!(self, Quote::Char | Quote::Byte)
    }
}

/// Does the `'` at `at` start a character literal, rather than a
/// lifetime? It does if it's followed by one character and a `'`, or by
/// an escape: `'a'`, `'é'`, `'\n'`, as against `'a`.
fn is_char_literal(src: &str, at: usize) -> bool {
    let rest = &src[at + 1..];
    let mut chars = rest.chars();
    match chars.next() {
        Some('\\') => true,
        Some('\'') => true,
        Some(c) => chars.next() == Some('\'') || !(c.is_ascii_alphanumeric() || c == '_'),
        None => false,
    }
}

/// The quoted literal starting with the quote at `at`: what it holds,
/// with its escapes worked out, and where it ends, after the closing
/// quote. Each escape means one thing: `\u{..}` a character, in text,
/// written as UTF-8, and `\x..` a byte, in a byte literal.
fn quoted(src: &str, at: usize, kind: Quote) -> Result<(Vec<u8>, usize), Error> {
    let bytes = src.as_bytes();
    let quote = bytes[at];
    let what = match kind {
        Quote::Str => "string literal",
        Quote::ByteStr => "byte string literal",
        Quote::Char => "character literal",
        Quote::Byte => "byte literal",
    };
    let start = if kind.bytes() { at - 1 } else { at };
    let mut out = Vec::new();
    let mut count = 0;
    let mut i = at + 1;
    loop {
        let Some(c) = src[i..].chars().next() else {
            return Err(Error::syntax(
                Span::new(start, start + 1),
                format!("this {what} never ends"),
            ));
        };
        if c as u32 == u32::from(quote) {
            i += 1;
            break;
        }
        if kind.one() && matches!(c, '\n' | '\r' | '\t') {
            return Err(Error::syntax(
                Span::new(i, i + 1),
                format!(
                    "a {what} can't hold a raw tab or line break; write it as an escape, like `\\n`"
                ),
            ));
        }
        count += 1;
        if c != '\\' {
            if kind.bytes() && !c.is_ascii() {
                return Err(Error::syntax(
                    Span::new(i, i + c.len_utf8()),
                    "a byte literal holds only ASCII characters; write other bytes as `\\x..`",
                ));
            }
            let mut buf = [0; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            i += c.len_utf8();
            continue;
        }
        let esc = i;
        let Some(e) = src[i + 1..].chars().next() else {
            i += 1;
            continue;
        };
        i += 1 + e.len_utf8();
        let simple = match e {
            'n' => Some(b'\n'),
            'r' => Some(b'\r'),
            't' => Some(b'\t'),
            '\\' => Some(b'\\'),
            '0' => Some(0),
            '\'' => Some(b'\''),
            '"' => Some(b'"'),
            _ => None,
        };
        if let Some(b) = simple {
            out.push(b);
            continue;
        }
        match (e, kind.bytes()) {
            ('x', true) => {
                let hex = src
                    .get(i..i + 2)
                    .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()));
                let Some(hex) = hex else {
                    return Err(Error::syntax(
                        Span::new(esc, i),
                        "`\\x` takes two hexadecimal digits, as in `\\x7F`",
                    ));
                };
                out.push(u8::from_str_radix(hex, 16).expect("hex digits"));
                i += 2;
            }
            ('x', false) => {
                return Err(Error::syntax(
                    Span::new(esc, i),
                    "`\\x` escapes are for byte literals; write a character as `\\u{..}`",
                ));
            }
            ('u', true) => {
                return Err(Error::syntax(
                    Span::new(esc, i),
                    "`\\u{..}` escapes are for characters; write a byte as `\\x..`",
                ));
            }
            ('u', false) => {
                let close = src[i..].find('}').map(|n| i + n);
                let digits = match close {
                    Some(close) if src[i..].starts_with('{') => &src[i + 1..close],
                    _ => "",
                };
                if digits.is_empty()
                    || digits.len() > 6
                    || !digits.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(Error::syntax(
                        Span::new(esc, i),
                        "`\\u` takes one to six hexadecimal digits in braces, as in `\\u{20AC}`",
                    ));
                }
                let end = close.expect("found") + 1;
                let n = u32::from_str_radix(digits, 16).expect("hex digits");
                let Some(ch) = char::from_u32(n) else {
                    return Err(Error::syntax(
                        Span::new(esc, end),
                        format!("`\\u{{{digits}}}` isn't a Unicode scalar value"),
                    ));
                };
                let mut buf = [0; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                i = end;
            }
            _ => {
                return Err(Error::syntax(
                    Span::new(esc, i),
                    format!("unknown escape `\\{e}`"),
                ));
            }
        }
    }
    if kind.one() && count != 1 {
        return Err(Error::syntax(
            Span::new(start, i),
            format!("a {what} holds exactly one character"),
        ));
    }
    Ok((out, i))
}

/// Character `c` as a literal: `'a'`, `'\n'`, `'\u{e000}'`.
pub fn char_literal(c: char) -> String {
    match c {
        '"' => "'\"'".to_string(),
        '\0' => "'\\0'".to_string(),
        c => format!("'{}'", c.escape_debug()),
    }
}

/// String `s` as a literal: `"a\tb"`.
pub fn str_literal(s: &str) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '\'' => out.push('\''),
            '\0' => out.push_str("\\0"),
            c => out.extend(c.escape_debug()),
        }
    }
    out.push('"');
    out
}

/// Byte `b` as a literal: `b'a'`, `b'\xFF'`.
pub fn byte_literal(b: u8) -> String {
    format!("b'{}'", escape_byte(b, b'\''))
}

/// Bytes `bs` as a byte string literal: `b"a\xFF"`.
pub fn byte_str_literal(bs: &[u8]) -> String {
    let body: String = bs.iter().map(|&b| escape_byte(b, b'"')).collect();
    format!("b\"{body}\"")
}

/// Byte `b` as it's written in a byte literal quoted with `quote`.
fn escape_byte(b: u8, quote: u8) -> String {
    match b {
        b'\n' => "\\n".into(),
        b'\r' => "\\r".into(),
        b'\t' => "\\t".into(),
        b'\\' => "\\\\".into(),
        0 => "\\0".into(),
        b if b == quote => format!("\\{}", b as char),
        b' '..=b'~' => (b as char).to_string(),
        b => format!("\\x{b:02X}"),
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

/// A `// ...` comment. The parser never sees these; `t fmt` keeps them.
/// A `///` or `//!` doc comment is a token instead (`Tok::Doc`,
/// `Tok::InnerDoc`).
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
            let text = &src[start..end];
            match (text.strip_prefix("///"), text.strip_prefix("//!")) {
                (Some(text), _) if !text.starts_with('/') => toks.push(Token {
                    tok: Tok::Doc(text.to_string()),
                    span,
                }),
                (_, Some(text)) => toks.push(Token {
                    tok: Tok::InnerDoc(text.to_string()),
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
        } else if c == b'"' {
            let (text, end) = quoted(src, i, Quote::Str)?;
            i = end;
            // Its bytes are those of the source, which is UTF-8, and of
            // escapes of scalar values: UTF-8 too.
            Tok::Str(String::from_utf8(text).expect("UTF-8"))
        } else if c == b'b' && matches!(bytes.get(i + 1), Some(b'"' | b'\'')) {
            let (text, end) = match bytes[i + 1] {
                b'"' => quoted(src, i + 1, Quote::ByteStr)?,
                _ => quoted(src, i + 1, Quote::Byte)?,
            };
            i = end;
            match bytes[start + 1] {
                b'"' => Tok::ByteStr(text),
                _ => Tok::Byte(text[0]),
            }
        } else if c == b'\'' && is_char_literal(src, i) {
            let (text, end) = quoted(src, i, Quote::Char)?;
            i = end;
            let text = String::from_utf8(text).expect("UTF-8");
            Tok::Char(text.chars().next().expect("one character"))
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
                "export" => Tok::Export,
                "mod" => Tok::Mod,
                "use" => Tok::Use,
                "crate" => Tok::Crate,
                "super" => Tok::Super,
                "where" => Tok::Where,
                "ensures" => Tok::Ensures,
                "with" => Tok::With,
                "effect" => Tok::Effect,
                "sealed" => Tok::Sealed,
                "const" => Tok::Const,
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
            // `'ab'` is no lifetime, but a character literal of two.
            if bytes.get(i) == Some(&b'\'') {
                return Err(Error::syntax(
                    Span::new(start, i + 1),
                    "a character literal holds exactly one character",
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
                (b'?', _) => (Tok::Question, 1),
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

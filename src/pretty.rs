//! Prints a program back out as source, for `t mono` and `t fmt`. A
//! program without generics, nested functions or local types prints as T
//! that parses back to the same program; the names monomorphisation gives
//! its copies, like `id::<i64>`, and the symbols nested functions and local
//! types are lifted under, like `main::helper`, don't parse.
//!
//! The printer turns the AST into a `Doc`, which says where lines may
//! break, and `doc::render` decides where they do. For `t fmt` it also has
//! the source's comments and blank lines (`Trivia`), and puts them back as
//! it goes. See docs/fmt.md.

use crate::ast::*;
use crate::doc::{Doc, WIDTH, concat, flat, flat_width, group, if_break, nest, render, text};
use crate::error::Span;
use crate::lexer::Tok;
use crate::trivia::Trivia;
use std::fmt;

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut p = Printer::default();
        let doc = p.plain_items(self, ROOT);
        f.write_str(&render(&doc))
    }
}

/// `e` as source, on one line if it fits, for messages.
pub fn expr_text(e: &Expr) -> String {
    render(&Printer::default().expr(e, 0, Ctx::default()))
        .trim_end()
        .to_string()
}

/// Prints `program` with the comments and blank lines in `trivia`, and its
/// items in the order they're written.
pub fn format(program: &Program, trivia: Trivia) -> String {
    let mut p = Printer {
        trivia: Some(trivia),
    };
    let docs = program.mods[ROOT].doc.as_ref().map(inner_doc);
    let (mut doc, mut empty) = p.items(program, ROOT);
    if let Some(docs) = docs {
        doc = match empty {
            true => docs,
            false => docs + Doc::HardLine + Doc::HardLine + doc,
        };
        empty = false;
    }
    for (i, c) in p.dangling(usize::MAX, empty).into_iter().enumerate() {
        if i > 0 || !empty {
            doc = doc + Doc::HardLine;
        }
        doc = doc + c;
    }
    render(&doc)
}

/// A module's `//!` doc comment, a line each.
fn inner_doc(d: &DocComment) -> Doc {
    let mut doc = Doc::nil();
    for (i, l) in d.lines.iter().enumerate() {
        if i > 0 {
            doc = doc + Doc::HardLine;
        }
        doc = doc + text(format!("//!{}", l.text));
    }
    doc
}

/// The items of module `module`, each with where its name is in the
/// source: its types, traits, impls, functions, modules and `use`s, or, if
/// `sorted`, in the order they're written.
fn items_of(program: &Program, module: usize, sorted: bool) -> Vec<(usize, Item<'_>)> {
    let mods = program.mods.iter().enumerate().skip(1);
    let mut items: Vec<(usize, Item)> = program
        .types
        .iter()
        .filter(|d| d.module() == module)
        .map(|d| (d.name().span.start, Item::Type(d)))
        .chain(
            program
                .traits
                .iter()
                .filter(|t| t.module == module)
                .map(|t| (t.span.start, Item::Trait(t))),
        )
        .chain(
            program
                .impls
                .iter()
                .filter(|d| d.module == module)
                .map(|i| (i.span.start, Item::Impl(i))),
        )
        .chain(
            program
                .fns
                .iter()
                .filter(|f| f.module == module)
                .map(|f| (f.name.span.start, Item::Fn(f))),
        )
        .chain(
            mods.filter(|(_, m)| m.parent == Some(module))
                .map(|(i, m)| (m.name.span.start, Item::Mod(i, m))),
        )
        .chain(
            program
                .uses
                .iter()
                .filter(|u| u.module == module)
                .map(|u| (u.span.start, Item::Use(u))),
        )
        .collect();
    if sorted {
        items.sort_by_key(|(pos, _)| *pos);
    }
    items
}

#[derive(Clone, Copy)]
enum Item<'a> {
    Type(&'a TypeDecl),
    Trait(&'a TraitDecl),
    Impl(&'a ImplDecl),
    Fn(&'a FnDecl),
    /// With its index in `Program::mods`.
    Mod(usize, &'a ModDecl),
    Use(&'a UseDecl),
}

/// How tightly a binary operator binds, as in the parser.
fn prec(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => CMP,
        BinOp::Add | BinOp::Sub => 4,
        BinOp::Mul | BinOp::Div | BinOp::Rem => 5,
        // As in the parser, though which comes first of these and the
        // others above needs parentheses: see `BinOp::nests_in`.
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => 4,
    }
}

const CMP: u8 = 3;

/// What `min` an operand `e` of `op` needs: more than any, so
/// parentheses, if it's an operator that can't be `op`'s operand without
/// them (`BinOp::nests_in`).
fn apart(e: &Expr, op: BinOp, min: u8) -> u8 {
    match e.kind {
        ExprKind::Binary(inner, ..) if !inner.nests_in(op) => UNARY + 1,
        _ => min,
    }
}

/// `as`: tighter than any binary operator, looser than a unary one.
const AS: u8 = 6;

/// Binds tighter than any binary operator or `as`: unary operators, calls
/// and everything else.
const UNARY: u8 = 7;

fn block_like(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::If(..)
            | ExprKind::IfLet(..)
            | ExprKind::Match(..)
            | ExprKind::While(..)
            | ExprKind::WhileLet(..)
            | ExprKind::Loop(..)
            | ExprKind::For(..)
            | ExprKind::Block(..)
            | ExprKind::Unsafe(..)
    )
}

/// Does `e` end in a `}` when printed? Then it needs parentheses before
/// the `else` of a `let .. else`, which would otherwise be read as an
/// `if`'s.
fn ends_in_brace(e: &Expr) -> bool {
    match &e.kind {
        _ if block_like(e) => true,
        ExprKind::StructLit(..) => true,
        ExprKind::Lambda(l) => ends_in_brace(&l.body),
        ExprKind::Binary(_, _, r) | ExprKind::Assign(_, _, r) | ExprKind::Range(_, r) => {
            ends_in_brace(r)
        }
        ExprKind::Unary(_, x) => ends_in_brace(x),
        ExprKind::Return(Some(x)) | ExprKind::Break(_, Some(x)) => ends_in_brace(x),
        _ => false,
    }
}

/// Is `e` an `&&` or `||`, which needs parentheses as what an `if let`,
/// `while let` or `let .. else` matches?
fn chain_like(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Binary(BinOp::And | BinOp::Or, ..))
}

/// `'a: ` before a loop, if it has a label.
fn label_doc(label: &Option<Label>) -> Doc {
    match label {
        Some(l) => text(format!("{}: ", l.name.name)),
        None => Doc::nil(),
    }
}

/// Is the last of `args` worth breaking inside rather than between the
/// arguments? See `Doc::Hug`.
fn huggable(args: &[Expr]) -> bool {
    let Some(last) = args.last() else {
        return false;
    };
    match &last.kind {
        ExprKind::If(..)
        | ExprKind::IfLet(..)
        | ExprKind::Match(..)
        | ExprKind::Block(..)
        | ExprKind::Unsafe(..) => true,
        ExprKind::StructLit(_, _, inits) => !inits.is_empty(),
        ExprKind::Array(items) => !items.is_empty(),
        ExprKind::Lambda(l) => matches!(l.body.kind, ExprKind::Block(..)),
        // `f(g(..))`, but not `f(x, g(..))`, which would hide `x` at the end
        // of the line.
        ExprKind::Call(..) | ExprKind::MethodCall(..) => args.len() == 1,
        ExprKind::Variant(_, _, _, args_, ..) => args.len() == 1 && !args_.is_empty(),
        _ => false,
    }
}

/// Where an expression is.
#[derive(Clone, Copy, Default)]
struct Ctx {
    /// In the head of an `if` or `match`, where a struct literal needs
    /// parentheses.
    no_struct: bool,
    /// At the start of a statement, where an `if`, `match` or block ends
    /// the statement, so one that's only the start of it (`(if c { 1 } else
    /// { 2 }) - x`) needs parentheses.
    stmt: bool,
}

impl Ctx {
    /// The same place, but not the start of a statement any more.
    fn inner(self) -> Ctx {
        Ctx {
            stmt: false,
            ..self
        }
    }
}

#[derive(Default)]
struct Printer<'s> {
    /// The source's comments and blank lines, when formatting it.
    trivia: Option<Trivia<'s>>,
}

/// The delimiters of a list, and whether there's a space inside them when
/// it's on one line: `{ x: 1 }`, but `(1, 2)`.
struct Delims(&'static str, &'static str, bool);

const PARENS: Delims = Delims("(", ")", false);
const ANGLES: Delims = Delims("<", ">", false);
const BRACES: Delims = Delims("{", "}", true);
const BRACKETS: Delims = Delims("[", "]", false);
const PIPES: Delims = Delims("|", "|", false);

impl Printer<'_> {
    /// What goes before something that starts at `pos` in the source and
    /// starts a line: an item, statement, arm or list element. That's the
    /// comments before it, each on a line of its own, and a blank line if
    /// there was one, unless it's `first` in its block or list.
    fn element(&mut self, pos: usize, first: bool) -> Doc {
        let Some(t) = &mut self.trivia else {
            return Doc::nil();
        };
        let mut doc = Doc::nil();
        let mut first = first;
        for c in t.leading(pos) {
            if c.blank && !first {
                doc = doc + Doc::HardLine;
            }
            doc = doc + c.text + Doc::HardLine;
            first = false;
        }
        if t.blank_before(pos) && !first {
            doc = doc + Doc::HardLine;
        }
        doc
    }

    /// The doc comment and attributes of something that starts at `pos` in
    /// the source (after them), each line on a line of its own, with any
    /// comments between them and it. The doc comment goes first, wherever
    /// it was.
    fn lead(&mut self, doc: Option<&DocComment>, allows: &[Allow], pos: usize) -> Doc {
        self.lead_derives(doc, allows, &[], pos)
    }

    /// `lead`, with `#[derive(..)]`s after the `#[allow(..)]`s.
    fn lead_derives(
        &mut self,
        doc: Option<&DocComment>,
        allows: &[Allow],
        derives: &[Derive],
        pos: usize,
    ) -> Doc {
        if doc.is_none() && allows.is_empty() && derives.is_empty() {
            return Doc::nil();
        }
        let mut out = concat(
            doc.iter()
                .flat_map(|d| &d.lines)
                .map(|l| text(format!("///{}", l.text)) + Doc::HardLine),
        );
        let no_wheres = Vec::new();
        let attrs = allows
            .iter()
            .map(|a| ("allow", a.span, &a.lints, &no_wheres));
        let attrs = attrs.chain(
            derives
                .iter()
                .map(|d| ("derive", d.span, &d.names, &d.wheres)),
        );
        for (attr, span, names, wheres) in attrs {
            let names: Vec<String> = names
                .iter()
                .enumerate()
                .map(|(i, l)| match wheres.get(i) {
                    Some(Some(w)) => format!("{} {}", l.name, where_clause(w)),
                    _ => l.name.clone(),
                })
                .collect();
            let attr = format!("#[{attr}({})]", names.join(", "));
            out = out + self.element(span.start, true) + attr + self.trailing(span.end);
            out = out + Doc::HardLine;
        }
        out + self.element(pos, true)
    }

    /// The comment after something that ends at `end`, if it's on the same
    /// line, and those lined up under it.
    fn trailing(&mut self, end: usize) -> Doc {
        let Some(t) = self.trivia.as_mut().and_then(|t| t.trailing(end)) else {
            return Doc::nil();
        };
        let mut doc = Doc::Comment(t.text);
        for more in t.more {
            doc = doc + Doc::HardLine + Doc::Comment(more);
        }
        doc
    }

    /// The comments left before `close`, the end of a block or list, each to
    /// go on a line of its own.
    fn dangling(&mut self, close: usize, first: bool) -> Vec<Doc> {
        let Some(t) = &mut self.trivia else {
            return Vec::new();
        };
        let mut first = first;
        t.leading(close)
            .into_iter()
            .map(|c| {
                let blank = c.blank && !std::mem::take(&mut first);
                let doc = text(c.text) + Doc::BreakParent;
                match blank {
                    true => Doc::HardLine + doc,
                    false => doc,
                }
            })
            .collect()
    }

    /// Are there comments to print before `pos`?
    fn comments_before(&self, pos: usize) -> bool {
        self.trivia.as_ref().is_some_and(|t| t.any_before(pos))
    }

    /// An integer literal or literal pattern at `span`, as the source
    /// writes it when formatting it, `0xFF`, and else as `plain` says.
    fn spelled(&self, span: Span, plain: String) -> Doc {
        text(
            self.trivia
                .as_ref()
                .and_then(|t| t.spelled(span))
                .unwrap_or(plain),
        )
    }

    /// Where the first `tok` at or after `pos` starts in the source.
    fn find(&self, pos: usize, tok: &Tok) -> usize {
        self.trivia.as_ref().map_or(0, |t| t.find(pos, tok).start)
    }

    /// Where the token before the one at `pos` starts.
    fn before(&self, pos: usize) -> usize {
        self.trivia.as_ref().map_or(0, |t| t.before(pos).start)
    }

    /// Where the `close` after the last of `ends` is, or else the first one
    /// after `open`: the end of a list.
    fn close(&self, open: usize, ends: Option<usize>, close: &Tok) -> usize {
        self.find(ends.unwrap_or(open), close)
    }

    /// `items`, separated by commas: all on one line, or else one per line
    /// with a comma after the last too. `close_pos` is where the closing
    /// delimiter is in the source, and `rest` goes after the last item
    /// (`..` in a struct pattern).
    fn list<T>(
        &mut self,
        Delims(open, close, spaced): Delims,
        close_pos: usize,
        items: &[T],
        span: impl Fn(&T) -> (usize, usize),
        rest: Option<(usize, &str)>,
        mut item: impl FnMut(&mut Self, &T) -> Doc,
    ) -> Doc {
        if items.is_empty() && rest.is_none() && !self.comments_before(close_pos) {
            return text(format!("{open}{close}"));
        }
        let mut entries = Vec::new();
        for (i, x) in items.iter().enumerate() {
            let (start, end) = span(x);
            let lead = self.element(start, i == 0);
            let doc = item(self, x);
            let comma = match i + 1 == items.len() && rest.is_none() {
                true => if_break(",", ""),
                false => text(","),
            };
            entries.push(lead + doc + comma + self.trailing(end));
        }
        if let Some((pos, rest)) = rest {
            entries.push(self.element(pos, items.is_empty()) + rest);
        }
        let first = entries.is_empty();
        entries.extend(self.dangling(close_pos, first));
        let line = || if spaced { Doc::Line } else { Doc::SoftLine };
        let inner = concat(
            entries
                .into_iter()
                .enumerate()
                .map(|(i, e)| if i == 0 { line() } else { Doc::Line } + e),
        );
        group(text(open) + nest(inner) + line() + close)
    }

    /// The items of module `module`, for `t fmt`, and whether it has none.
    fn items(&mut self, program: &Program, module: usize) -> (Doc, bool) {
        let items = items_of(program, module, true);
        let mut doc = Doc::nil();
        for (i, &(pos, item)) in items.iter().enumerate() {
            if i > 0 {
                doc = doc + Doc::HardLine;
            }
            // It starts at its doc comment or attributes, or else at the
            // `pub`, `struct`, `enum` or `fn` before the name.
            let start = match item {
                Item::Type(d) => {
                    let start = type_lead_start(d, self.before(pos));
                    d.vis().span().map_or(start, |p| start.min(p.start))
                }
                Item::Fn(f) => lead_start(f.doc.as_ref(), &f.allows, fn_start(f)),
                Item::Trait(t) => {
                    let start = t.vis.span().map_or(t.span.start, |p| p.start);
                    let start = t.intrinsic.map_or(start, |i| i.start.min(start));
                    lead_start(t.doc.as_ref(), &t.allows, start)
                }
                Item::Impl(d) => lead_start(d.doc.as_ref(), &d.allows, d.span.start),
                Item::Mod(_, m) => lead_start(None, &m.allows, m.span.start),
                Item::Use(u) => u.span.start,
            };
            doc = doc + self.element(start, i == 0);
            let end = match item {
                Item::Type(_) => self.find(pos, &Tok::RBrace) + 1,
                Item::Fn(f) => f.body.span.end,
                Item::Trait(t) => t.span.end,
                Item::Impl(d) => d.span.end,
                Item::Mod(_, m) => m.span.end,
                Item::Use(u) => u.span.end,
            };
            let item = match item {
                Item::Mod(i, m) => self.mod_decl(program, i, m),
                item => self.item(item),
            };
            doc = doc + item + self.trailing(end);
        }
        (doc, items.is_empty())
    }

    /// The items of module `module`, for `t mono` and `Display`: with no
    /// comments, a blank line between each two.
    fn plain_items(&mut self, program: &Program, module: usize) -> Doc {
        let mut doc = Doc::nil();
        for (i, (_, item)) in items_of(program, module, false).into_iter().enumerate() {
            if i > 0 {
                doc = doc + Doc::HardLine + Doc::HardLine;
            }
            doc = doc
                + match item {
                    Item::Mod(i, m) => {
                        let body = nest(Doc::HardLine + self.plain_items(program, i));
                        self.lead(None, &m.allows, m.span.start)
                            + vis(m.vis)
                            + "mod "
                            + &m.name.name
                            + " {"
                            + body
                            + Doc::HardLine
                            + "}"
                    }
                    item => self.item(item),
                };
        }
        doc
    }

    /// `mod name { .. }` or `mod name;`, for `t fmt`.
    fn mod_decl(&mut self, program: &Program, i: usize, m: &ModDecl) -> Doc {
        // Its doc comment is `//!` lines in it, first.
        let lead = self.lead(None, &m.allows, m.span.start);
        if m.in_file {
            return lead + vis(m.vis) + "mod " + &m.name.name + ";";
        }
        let head = lead + vis(m.vis) + "mod " + &m.name.name + " ";
        let open = self.find(m.name.span.end, &Tok::LBrace);
        let close = m.span.end - 1;
        let (items, mut empty) = self.items(program, i);
        if empty && m.doc.is_none() && !self.comments_before(close) {
            return head + "{}";
        }
        let mut body = self.trailing(open + 1);
        if let Some(d) = &m.doc {
            body = body + Doc::HardLine + inner_doc(d);
            if !empty {
                body = body + Doc::HardLine;
            }
        }
        if !empty {
            body = body + Doc::HardLine + items;
        }
        empty = empty && m.doc.is_none();
        for c in self.dangling(close, empty) {
            body = body + Doc::HardLine + c;
        }
        head + "{" + nest(body) + Doc::HardLine + "}"
    }

    fn item(&mut self, item: Item) -> Doc {
        match item {
            Item::Type(d) => self.type_decl(d),
            Item::Trait(t) => self.trait_decl(t),
            Item::Impl(d) => self.impl_decl(d),
            Item::Fn(f) => self.fn_decl(f),
            Item::Use(u) => vis(u.vis) + "use " + use_tree(&u.tree) + ";",
            Item::Mod(..) => unreachable!("printed by `items` or `plain_items`"),
        }
    }

    /// `trait Ord: Eq { .. }`
    fn trait_decl(&mut self, t: &TraitDecl) -> Doc {
        let mut doc = self.lead(t.doc.as_ref(), &t.allows, t.span.start);
        if let Some(at) = t.intrinsic {
            doc = doc + "#[intrinsic]" + self.trailing(at.end) + Doc::HardLine;
            doc = doc + self.element(self.before(t.span.start), true);
        }
        let supers: Vec<&str> = t.supers.iter().map(|s| s.name.as_str()).collect();
        let supers = match supers.is_empty() {
            true => String::new(),
            false => format!(": {}", supers.join(" + ")),
        };
        let open = self.find(t.name.span.end, &Tok::LBrace);
        let mut items: Vec<BodyItem> = t.assoc.iter().map(BodyItem::Decl).collect();
        items.extend(t.effects.iter().map(BodyItem::EffectDecl));
        items.extend(t.methods.iter().map(|m| BodyItem::Fn(&m.decl, m.provided)));
        let body = self.methods(open, t.span.end - 1, items);
        let sealed = if t.sealed.is_some() { "sealed " } else { "" };
        doc + vis(t.vis) + sealed + "trait " + &t.name.name + supers + " " + body
    }

    /// `impl<T: Show> Show for Option<T> { .. }` or `impl Point { .. }`
    fn impl_decl(&mut self, d: &ImplDecl) -> Doc {
        let doc = self.lead(d.doc.as_ref(), &d.allows, d.span.start);
        let generics = self.generics(d.span.start + 4, &d.generics);
        let header = match &d.trait_name {
            Some(t) => text(format!(" {} for ", t.name)),
            None => text(" "),
        };
        let open = self.find(d.self_ty.span.end, &Tok::LBrace);
        let mut items: Vec<BodyItem> = d.assoc.iter().map(BodyItem::Def).collect();
        items.extend(d.effects.iter().map(BodyItem::EffectDef));
        items.extend(d.methods.iter().map(|f| BodyItem::Fn(f, true)));
        let body = self.methods(open, d.span.end - 1, items);
        // A type short enough stays on the line, so a long header breaks
        // its `<..>` instead, as a function breaks its parameters before
        // its result.
        let ty = self.ty(&d.self_ty);
        let short = |f: &Doc| flat_width(f).is_some_and(|w| w <= WIDTH / 2);
        let ty = flat(&ty).filter(short).unwrap_or(ty);
        let wheres = self.wheres(None, &d.generics, &[], true);
        doc + "impl" + generics + header + ty + wheres + body
    }

    /// The `{ .. }` of a trait or impl, with the `{` at `open` and the `}`
    /// at `close` in the source: its associated types and methods, in the
    /// order written, one after another.
    fn methods(&mut self, open: usize, close: usize, mut methods: Vec<BodyItem>) -> Doc {
        if methods.is_empty() && !self.comments_before(close) {
            return text("{}");
        }
        methods.sort_by_key(|m| m.span().start);
        let mut body = self.trailing(open + 1);
        for (i, item) in methods.iter().enumerate() {
            let (doc, allows, span) = match item {
                BodyItem::Fn(f, _) => (
                    f.doc.as_ref(),
                    &f.allows[..],
                    Span::new(fn_start(f), f.span.end),
                ),
                BodyItem::Decl(a) => (a.doc.as_ref(), &[][..], a.span),
                BodyItem::Def(a) => (a.doc.as_ref(), &[][..], a.span),
                BodyItem::EffectDecl(a) => (a.doc.as_ref(), &[][..], a.span),
                BodyItem::EffectDef(a) => (a.doc.as_ref(), &[][..], a.span),
            };
            let start = lead_start(doc, allows, span.start);
            body = body + Doc::HardLine + self.element(start, i == 0);
            let doc = match *item {
                BodyItem::Fn(f, true) => self.fn_decl(f),
                BodyItem::Fn(f, false) => {
                    self.signature(f)
                        + self.wheres(f.effects.as_ref(), &f.generics, &f.ensures, false)
                        + ";"
                }
                BodyItem::Decl(a) => {
                    let lead = self.lead(a.doc.as_ref(), &[], a.span.start);
                    let bounds: Vec<&str> = a.bounds.iter().map(|b| b.name.as_str()).collect();
                    let bounds = match bounds.is_empty() {
                        true => String::new(),
                        false => format!(": {}", bounds.join(" + ")),
                    };
                    lead + "type " + &a.name.name + bounds + ";"
                }
                BodyItem::Def(a) => {
                    let lead = self.lead(a.doc.as_ref(), &[], a.span.start);
                    lead + "type " + &a.name.name + " = " + self.ty(&a.ty) + ";"
                }
                BodyItem::EffectDecl(a) => {
                    let lead = self.lead(a.doc.as_ref(), &[], a.span.start);
                    lead + "effect " + &a.name.name + ";"
                }
                BodyItem::EffectDef(a) => {
                    let lead = self.lead(a.doc.as_ref(), &[], a.span.start);
                    lead + "effect " + &a.name.name + " = " + self.effects(&a.effects) + ";"
                }
            };
            body = body + doc + self.trailing(span.end);
        }
        for c in self.dangling(close, methods.is_empty()) {
            body = body + Doc::HardLine + c;
        }
        text("{") + nest(body) + Doc::HardLine + "}"
    }

    /// `<'a, T: Show + 'a>` after the name of a generic function, struct or
    /// enum, or after `impl`.
    fn generics(&mut self, after: usize, g: &Generics) -> Doc {
        if g.is_empty() {
            return Doc::nil();
        }
        let params: Vec<&Ident> = g.lifetimes.iter().chain(&g.types).collect();
        // Each with the end of its bounds, if it has any.
        let end = |p: &Ident| {
            if let Some((_, t)) = g.consts.iter().find(|(id, _)| *id == p.id) {
                return t.span.end;
            }
            let bounds = g.bounds_of(&p.name);
            bounds.last().map_or(p.span.end, |b| b.span.end)
        };
        let close = self.close(after, params.last().map(|p| end(p)), &Tok::Gt);
        self.list(
            ANGLES,
            close,
            &params,
            |p| (p.span.start, end(p)),
            None,
            |pp, p| {
                let mut out = match g.consts.iter().find(|(id, _)| *id == p.id) {
                    Some((_, t)) => text("const ") + text(&p.name) + ": " + pp.ty(t),
                    None if g.is_effect(p) => text("effect ") + text(&p.name),
                    None => text(&p.name),
                };
                for (i, b) in g.bounds_of(&p.name).into_iter().enumerate() {
                    out = out + if i == 0 { ": " } else { " + " } + pp.bound(b);
                }
                out
            },
        )
    }

    /// `Show`, `Iter<Item = i64>` or `Fn(i64) -> i64`.
    fn bound(&mut self, b: &BoundItem) -> Doc {
        let mut out = text(&b.name.name);
        if let Some((params, ret)) = &b.sig {
            let open = self.find(b.name.span.end, &Tok::LParen);
            let last = params.last().map(|p| p.span.end);
            let close = self.close(open, last, &Tok::RParen);
            out = out + self.types(PARENS, close, params);
            if let Some(ret) = ret {
                out = out + " -> " + self.ty(ret);
            }
            if let Some(e) = &b.effects {
                out = out + " " + self.effects(e);
            }
            return out;
        }
        if b.assoc.is_empty() {
            return out;
        }
        let mut args = Doc::nil();
        for (j, a) in b.assoc.iter().enumerate() {
            if j > 0 {
                args = args + ", ";
            }
            args = args + &a.name.name;
            args = match &a.kind {
                AssocBoundKind::Eq(t) => args + " = " + self.ty(t),
                AssocBoundKind::Bounds(ts) => {
                    let ts: Vec<&str> = ts.iter().map(|t| t.name.as_str()).collect();
                    args + ": " + ts.join(" + ")
                }
            };
        }
        out + "<" + args + ">"
    }

    fn type_decl(&mut self, d: &TypeDecl) -> Doc {
        let keyword = match d {
            TypeDecl::Struct(_) => "struct ",
            TypeDecl::Enum(_) => "enum ",
            TypeDecl::Alias(_) => "type ",
        };
        let name = d.name();
        if let TypeDecl::Alias(a) = d {
            let doc = self.lead_derives(d.doc(), d.allows(), &[], self.before(name.span.start));
            return doc
                + vis(d.vis())
                + keyword
                + &name.name
                + self.generics(name.span.end, d.generics())
                + " = "
                + self.ty(&a.ty)
                + ";";
        }
        let mut doc = self.lead_derives(
            d.doc(),
            d.allows(),
            d.derives(),
            self.before(name.span.start),
        );
        if let TypeDecl::Struct(StructDecl {
            intrinsic: Some(at),
            ..
        }) = d
        {
            doc = doc + "#[intrinsic]" + self.trailing(at.end) + Doc::HardLine;
            doc = doc + self.element(self.before(name.span.start), true);
        }
        let open = self.find(name.span.end, &Tok::LBrace);
        let close = self.find(open, &Tok::RBrace);
        if let Some(t) = &mut self.trivia {
            t.hold_docs(name.span.start, close);
        }
        let head = doc
            + vis(d.vis())
            + keyword
            + &name.name
            + self.generics(name.span.end, d.generics())
            + " ";
        let body = match d {
            TypeDecl::Struct(s) => self.list(
                BRACES,
                close,
                &s.fields,
                |f| {
                    let name = f.vis.span().unwrap_or(f.name.span).start;
                    let start = lead_start(f.doc.as_ref(), &f.allows, name);
                    (start, f.ty.span.end)
                },
                None,
                |p, f| {
                    let name = f.vis.span().unwrap_or(f.name.span).start;
                    p.lead(f.doc.as_ref(), &f.allows, name) + vis(f.vis) + p.param(f)
                },
            ),
            TypeDecl::Enum(e) => {
                let variants: Vec<(&Variant, usize)> = e
                    .variants
                    .iter()
                    .map(|v| match v.fields.last() {
                        Some(t) => (v, self.find(t.span.end, &Tok::RParen) + 1),
                        None => (v, v.name.span.end),
                    })
                    .collect();
                self.list(
                    BRACES,
                    close,
                    &variants,
                    |(v, end)| {
                        let start = lead_start(v.doc.as_ref(), &v.allows, v.name.span.start);
                        (start, *end)
                    },
                    None,
                    |p, (v, end)| {
                        let doc = p.lead(v.doc.as_ref(), &v.allows, v.name.span.start);
                        let fields = match v.fields.is_empty() {
                            true => Doc::nil(),
                            false => p.types(PARENS, end - 1, &v.fields),
                        };
                        doc + &v.name.name + fields
                    },
                )
            }
            TypeDecl::Alias(_) => unreachable!("returned above"),
        };
        head + body
    }

    /// `name: type`, or `mut name: type`. A receiver as it's written:
    /// `self`, `mut self`, `&self`, `&'a mut self`.
    fn param(&mut self, p: &Param) -> Doc {
        if p.name.name == "self" {
            let shorthand = match &p.ty.kind {
                TypeExprKind::Ref(lifetime, m, _) => {
                    let lifetime = lifetime.as_ref().map(|l| format!("{} ", l.name));
                    let m = if *m { "mut " } else { "" };
                    format!("&{}{m}self", lifetime.unwrap_or_default())
                }
                _ if p.mutable.is_some() => "mut self".to_string(),
                _ => "self".to_string(),
            };
            return text(shorthand);
        }
        let name = match p.mutable {
            Some(_) => format!("mut {}", p.name.name),
            None => p.name.name.clone(),
        };
        text(name) + ": " + self.ty(&p.ty)
    }

    fn fn_decl(&mut self, f: &FnDecl) -> Doc {
        match f.intrinsic {
            Some(_) => {
                self.signature(f)
                    + self.wheres(f.effects.as_ref(), &f.generics, &f.ensures, false)
                    + ";"
            }
            // In order: comments are printed as what's after them is.
            None => {
                let head = self.signature(f)
                    + self.wheres(f.effects.as_ref(), &f.generics, &f.ensures, true);
                head + self.expr(&f.body, 0, Ctx::default())
            }
        }
    }

    /// A function's or impl's `where` clause, from `g`, if it has one, on
    /// the signature's line if it fits there, and otherwise on lines of its
    /// own, one bound to a line, as rustfmt does: then a body's `{` goes on
    /// a line of its own too. With a space before the body after it, if
    /// there's a `body`.
    ///
    /// A function's `effects`, `with ..`, come first, and go on a line of
    /// their own with it; its `ensures` last, laid out as `where` is.
    fn wheres(
        &mut self,
        effects: Option<&EffectSet>,
        g: &Generics,
        ensures: &[Expr],
        body: bool,
    ) -> Doc {
        let after = if body { text(" ") } else { Doc::nil() };
        let with = effects.is_some();
        let effects = match effects {
            Some(e) => Doc::Line + self.effects(e),
            None => Doc::nil(),
        };
        let end = match body {
            true => Doc::Line,
            false => Doc::nil(),
        };
        let comma = Doc::IfBreak(Box::new(text(",")), Box::new(Doc::nil()));
        let has_ensures = !ensures.is_empty();
        let ensures = match ensures.is_empty() {
            true => Doc::nil(),
            false => {
                let mut preds = Doc::nil();
                for (i, e) in ensures.iter().enumerate() {
                    if i > 0 {
                        preds = preds + "," + Doc::Line;
                    }
                    preds = preds + self.expr(e, 0, Ctx::default());
                }
                let comma = match body {
                    true => comma.clone(),
                    false => Doc::nil(),
                };
                Doc::Line + "ensures" + nest(Doc::Line + preds + comma)
            }
        };
        if g.wheres.is_empty() {
            return match with || has_ensures {
                false => after,
                true => group(effects + ensures + end),
            };
        }
        let mut preds = Doc::nil();
        for (i, (subject, bounds)) in g.wheres.iter().enumerate() {
            if i > 0 {
                preds = preds + "," + Doc::Line;
            }
            preds = preds + self.ty(subject);
            for (j, b) in bounds.iter().enumerate() {
                preds = preds + if j == 0 { ": " } else { " + " } + self.bound(b);
            }
        }
        group(effects + Doc::Line + "where" + nest(Doc::Line + preds + comma) + ensures + end)
    }

    /// A function, up to its body.
    fn signature(&mut self, f: &FnDecl) -> Doc {
        let mut doc = self.lead(f.doc.as_ref(), &f.allows, fn_start(f));
        // `#[intrinsic]`, `#[measure]` and `#[test]`, in the order written.
        let test = f.test.map(|t| match t.panics {
            true => (t.span, "#[test(panics)]"),
            false => (t.span, "#[test]"),
        });
        let mut attrs: Vec<(Span, &str)> =
            [(f.intrinsic, "#[intrinsic]"), (f.measure, "#[measure]")]
                .into_iter()
                .filter_map(|(at, t)| Some((at?, t)))
                .chain(test)
                .collect();
        attrs.sort_by_key(|(at, _)| at.start);
        let after = f.vis.span().map_or(f.span.start, |p| p.start);
        for (i, (at, t)) in attrs.iter().enumerate() {
            let next = attrs.get(i + 1).map_or(after, |(n, _)| n.start);
            doc = doc + *t + self.trailing(at.end) + Doc::HardLine;
            doc = doc + self.element(next, true);
        }
        let doc = doc + vis(f.vis);
        let generics = self.generics(f.name.span.end, &f.generics);
        let open = self.find(f.name.span.end, &Tok::LParen);
        let close = self.close(open, f.params.last().map(|p| p.ty.span.end), &Tok::RParen);
        let params = self.list(
            PARENS,
            close,
            &f.params,
            |p| (param_start(p), p.ty.span.end),
            None,
            |p, param| p.param(param),
        );
        let ret = match f.ret.kind {
            TypeExprKind::Unit => Doc::nil(),
            // A refinement stays on one line, so the parameters break
            // first, rather than its predicate.
            TypeExprKind::Refined(..) => {
                let ty = self.ty(&f.ret);
                text(" -> ") + crate::doc::flat(&ty).unwrap_or(ty)
            }
            _ => text(" -> ") + self.ty(&f.ret),
        };
        // The return type goes in the parameters' group, so that they stay
        // on one line only if it fits after them too. Otherwise they break,
        // one to a line, and it breaks only if it doesn't fit after the `)`
        // either: not `-> Result<` with its arguments on lines of their own.
        let head = match params {
            Doc::Group { doc, hard } => Doc::Group {
                doc: Box::new(*doc + ret),
                hard,
            },
            params => params + ret,
        };
        doc + "fn " + &f.name.name + generics + head
    }

    /// `with panic`, `with {alloc, e}`, or `{}` after `effect e =`. One
    /// effect isn't in braces.
    fn effects(&mut self, e: &EffectSet) -> Doc {
        let items: Vec<Doc> = e.items.iter().map(|i| self.effect(i)).collect();
        let mut out = match e.with {
            true => text("with "),
            false => Doc::nil(),
        };
        if items.len() == 1 {
            return out + items.into_iter().next().expect("one");
        }
        out = out + "{";
        for (i, item) in items.into_iter().enumerate() {
            if i > 0 {
                out = out + ", ";
            }
            out = out + item;
        }
        out + "}"
    }

    fn effect(&mut self, e: &Effect) -> Doc {
        match &e.kind {
            EffectKind::Named(n) => text(&n.name),
            EffectKind::Proj(t, Some(tr), name) => {
                text("<") + self.ty(t) + " as " + &tr.name + ">::" + &name.name
            }
            EffectKind::Proj(t, None, name) => self.ty(t) + "::" + &name.name,
        }
    }

    /// Prints `e`, in parentheses if it binds less tightly than `min`, or if
    /// `ctx` needs them.
    fn expr(&mut self, e: &Expr, min: u8, ctx: Ctx) -> Doc {
        let own = match &e.kind {
            ExprKind::Binary(op, ..) => prec(*op),
            ExprKind::As(..) => AS,
            // A closure's body runs as far right as it can, and so do the
            // value of an assignment, `return` and `break`. Even without a
            // value they need parentheses as an operand: `return - 1`
            // returns `-1`.
            ExprKind::Lambda(_)
            | ExprKind::Assign(..)
            | ExprKind::Return(_)
            | ExprKind::Break(..)
            | ExprKind::Range(..) => 0,
            _ => UNARY,
        };
        let parens = own < min
            || (ctx.no_struct && matches!(e.kind, ExprKind::StructLit(..)))
            || (ctx.stmt && block_like(e));
        if parens {
            let inner = Ctx {
                no_struct: false,
                stmt: false,
            };
            return text("(") + self.expr_inner(e, inner) + ")";
        }
        self.expr_inner(e, ctx)
    }

    fn expr_inner(&mut self, e: &Expr, ctx: Ctx) -> Doc {
        match &e.kind {
            ExprKind::Int(n, None) => self.spelled(e.span, n.to_string()),
            ExprKind::Int(n, Some(t)) => self.spelled(e.span, format!("{n}{}", t.name())),
            ExprKind::Bool(b) => text(b.to_string()),
            ExprKind::Char(c) => self.spelled(e.span, crate::lexer::char_literal(*c)),
            ExprKind::Str(s) => self.spelled(e.span, crate::lexer::str_literal(s)),
            ExprKind::ByteStr(b) => self.spelled(e.span, crate::lexer::byte_str_literal(b)),
            ExprKind::Unit => text("()"),
            ExprKind::Var(name, targs) => text(name) + self.turbofish(targs),
            // There's no syntax for one: printed as the place it moves out
            // of, as how a closure captures isn't printed either.
            ExprKind::Move(x) => self.expr_inner(x, ctx),
            ExprKind::Unary(op, x) => text(op.to_string()) + self.expr(x, UNARY, ctx.inner()),
            ExprKind::As(x, t) => self.expr(x, AS, ctx.inner()) + " as " + self.ty(t),
            ExprKind::Binary(..) => self.binary(e, ctx),
            ExprKind::Call(f, args) => {
                // Something more than a name or a call needs parentheses to
                // be called: `(|x| x)(1)`, and `(s.f)(1)`, as `s.f(1)` is a
                // method call.
                let min = match f.unmoved().kind {
                    ExprKind::Var(..) | ExprKind::Call(..) | ExprKind::MethodCall(..) => UNARY,
                    _ => UNARY + 1,
                };
                self.expr(f, min, ctx) + self.args(f.span.end, args)
            }
            ExprKind::Field(x, field) => self.receiver(x, ctx) + "." + &field.name,
            ExprKind::Try(x, _) => self.receiver(x, ctx) + "?",
            ExprKind::Index(x, i, ..) => {
                self.receiver(x, ctx) + "[" + self.expr(i, 0, Ctx::default()) + "]"
            }
            ExprKind::MethodCall(x, method, targs, args) => {
                let end = targs.last().map_or(method.span.end, |t| t.span.end);
                self.receiver(x, ctx)
                    + "."
                    + &method.name
                    + self.turbofish(targs)
                    + self.args(end, args)
            }
            ExprKind::If(..) | ExprKind::IfLet(..) => self.if_chain(e),
            ExprKind::While(label, c, body) => {
                group(label_doc(label) + "while " + self.head(c) + self.block_or_expr(body))
            }
            ExprKind::WhileLet(label, c, body) => {
                group(label_doc(label) + "while " + self.let_cond(c) + self.block_or_expr(body))
            }
            ExprKind::Loop(label, body) => {
                group(label_doc(label) + "loop " + self.block_or_expr(body))
            }
            ExprKind::For(f) => group(
                label_doc(&f.label)
                    + "for "
                    + self.pattern(&f.pat)
                    + " in "
                    + self.head(&f.iter)
                    + self.block_or_expr(&f.body),
            ),
            // Its ends bind more tightly than it, whatever they are but an
            // assignment or a closure.
            ExprKind::Range(l, r) => {
                self.expr(l, 1, ctx.inner()) + ".." + self.expr(r, 1, ctx.inner())
            }
            ExprKind::Return(value) | ExprKind::Break(_, value) => {
                let mut head = match &e.kind {
                    ExprKind::Break(Some(label), _) => format!("break {}", label.name),
                    ExprKind::Break(None, _) => "break".to_string(),
                    _ => "return".to_string(),
                };
                match value {
                    Some(v) => {
                        head.push(' ');
                        text(head) + self.expr(v, 0, ctx.inner())
                    }
                    None => text(head),
                }
            }
            ExprKind::Continue(label) => match label {
                Some(label) => text(format!("continue {}", label.name)),
                None => text("continue"),
            },
            ExprKind::Assign(op, place, value) => {
                let op = op.map_or(String::new(), |op| op.to_string());
                let place = self.expr(place, 1, ctx);
                place + format!(" {op}= ") + self.expr(value, 0, ctx.inner())
            }
            ExprKind::Block(stmts, tail) => group(self.block(stmts, tail.as_deref(), e.span)),
            ExprKind::Unsafe(block) => group(text("unsafe ") + self.block_or_expr(block)),
            ExprKind::StructLit(name, targs, inits) => {
                let close = e.span.end - 1;
                let fields = self.list(
                    BRACES,
                    close,
                    inits,
                    |i| (i.name.span.start, i.value.span.end),
                    None,
                    |p, init| {
                        let shorthand = matches!(&init.value.unmoved().kind,
                            ExprKind::Var(v, t) if *v == init.name.name && t.is_empty());
                        match shorthand {
                            true => text(&init.name.name),
                            false => {
                                text(&init.name.name)
                                    + ": "
                                    + p.expr(&init.value, 0, Ctx::default())
                            }
                        }
                    },
                );
                text(&name.name) + self.turbofish(targs) + " " + fields
            }
            ExprKind::Variant(ty, targs, variant, args, parens) => {
                let head = text(&ty.name) + self.turbofish(targs) + "::" + &variant.name;
                match parens {
                    false => head,
                    true => head + self.args(variant.span.end, args),
                }
            }
            ExprKind::Array(items) => self.list(
                BRACKETS,
                e.span.end - 1,
                items,
                |a| (a.span.start, a.span.end),
                None,
                |p, a| p.expr(a, 0, Ctx::default()),
            ),
            ExprKind::Repeat(v, n) => group(
                text("[")
                    + self.expr(v, 0, Ctx::default())
                    + "; "
                    + self.expr(n, 0, Ctx::default())
                    + "]",
            ),
            ExprKind::Match(scrutinee, arms) => {
                let head = text("match ") + self.head(scrutinee);
                let close = e.span.end - 1;
                if arms.is_empty() && !self.comments_before(close) {
                    return head + "{}";
                }
                let open = self.find(scrutinee.span.end, &Tok::LBrace);
                let mut body = self.trailing(open + 1);
                for (i, arm) in arms.iter().enumerate() {
                    let lead = self.element(arm.pat.span.start, i == 0);
                    let arm_doc = self.arm(arm);
                    body = body
                        + Doc::HardLine
                        + lead
                        + arm_doc
                        + ","
                        + self.trailing(arm.body.span.end);
                }
                for c in self.dangling(close, arms.is_empty()) {
                    body = body + Doc::HardLine + c;
                }
                head + "{" + nest(body) + Doc::HardLine + "}"
            }
            ExprKind::Lambda(l) => {
                let params = match l.params.last() {
                    None => text("||"),
                    Some(last) => {
                        let last_end = last.ty.as_ref().map_or(last.name.span.end, |t| t.span.end);
                        let close = self.find(last_end, &Tok::Pipe);
                        self.list(
                            PIPES,
                            close,
                            &l.params,
                            |p| {
                                let end = p.ty.as_ref().map_or(p.name.span.end, |t| t.span.end);
                                (p.mutable.unwrap_or(p.name.span).start, end)
                            },
                            None,
                            |p, param| {
                                let name = match param.mutable {
                                    Some(_) => format!("mut {}", param.name.name),
                                    None => param.name.name.clone(),
                                };
                                match &param.ty {
                                    Some(t) => text(name) + ": " + p.ty(t),
                                    None => text(name),
                                }
                            },
                        )
                    }
                };
                let params = match l.moves {
                    Some(_) => text("move ") + params,
                    None => params,
                };
                match &l.ret {
                    Some(t) => {
                        let ret = self.ty(t);
                        params + " -> " + ret + " " + self.expr(&l.body, 0, Ctx::default())
                    }
                    None => params + " " + self.expr(&l.body, 0, ctx.inner()),
                }
            }
        }
    }

    /// A run of binary operators that bind equally tightly, like
    /// `a + b - c`, breaks before each operator, all or none:
    ///
    /// ```text
    /// a
    ///     + b
    ///     - c
    /// ```
    fn binary(&mut self, e: &Expr, ctx: Ctx) -> Doc {
        let ExprKind::Binary(op, ..) = e.kind else {
            unreachable!()
        };
        let p = prec(op);
        // Collect the run down its left side: ((a + b) - c) is a, b, c.
        let mut rights = Vec::new();
        let mut left = e;
        while let ExprKind::Binary(inner, l, r) = &left.kind
            && prec(*inner) == p
            && inner.nests_in(op)
            && (p != CMP || rights.is_empty())
        {
            rights.push((*inner, &**r));
            left = l;
        }
        rights.reverse();
        // Left-associative, so the right side must bind tighter, and so must
        // the left side of a comparison, as they don't chain. `&&` inside
        // `||` gets parentheses, though it needn't: `(a && b) || c`.
        let min = match op {
            BinOp::Or => prec(BinOp::And) + 1,
            _ if p == CMP => p + 1,
            _ => p,
        };
        let first = self.expr(left, apart(left, op, min), ctx);
        let mut rest = Doc::nil();
        let inner = Ctx { stmt: false, ..ctx };
        for (op, r) in rights {
            let min = if op == BinOp::Or { min } else { p + 1 };
            rest = rest + Doc::Line + op.to_string() + " " + self.expr(r, apart(r, op, min), inner);
        }
        group(first + nest(rest))
    }

    /// What's before the `.` of a field or method call, in parentheses if
    /// it binds less tightly than the `.`, or would be hard to read without
    /// them: `(*r).x`, `(a + b).len()`, `(P { x: 1 }).x`.
    fn receiver(&mut self, x: &Expr, ctx: Ctx) -> Doc {
        let min = match x.kind {
            ExprKind::Unary(..)
            | ExprKind::StructLit(..)
            | ExprKind::Block(..)
            | ExprKind::Unsafe(..)
            | ExprKind::If(..)
            | ExprKind::IfLet(..)
            | ExprKind::Match(..)
            | ExprKind::While(..)
            | ExprKind::WhileLet(..)
            | ExprKind::Loop(..)
            | ExprKind::For(..) => UNARY + 1,
            _ => UNARY,
        };
        self.expr(x, min, ctx)
    }

    /// Call arguments, after the callee or variant that ends at `after`.
    fn args(&mut self, after: usize, args: &[Expr]) -> Doc {
        let close = self.close(after, args.last().map(|a| a.span.end), &Tok::RParen);
        let span = |a: &Expr| (a.span.start, a.span.end);
        if self.comments_before(close) || !huggable(args) {
            return self.list(PARENS, close, args, span, None, |p, a| {
                p.expr(a, 0, Ctx::default())
            });
        }
        let docs: Vec<Doc> = args
            .iter()
            .map(|a| self.expr(a, 0, Ctx::default()))
            .collect();
        let indexes: Vec<usize> = (0..args.len()).collect();
        let normal = self.list(
            PARENS,
            close,
            &indexes,
            |&i| span(&args[i]),
            None,
            |_, &i| docs[i].clone(),
        );
        let (last, init) = docs.split_last().unwrap();
        let prefix = concat(init.iter().map(|d| d.clone() + ", "));
        Doc::Hug {
            normal: Box::new(normal),
            prefix: Box::new(text("(") + prefix),
            last: Box::new(last.clone()),
            close: Box::new(text(")")),
        }
    }

    /// `if a { .. } else if b { .. } else { .. }`, in one group: all on one
    /// line, or every block broken.
    fn if_chain(&mut self, e: &Expr) -> Doc {
        let mut doc = Doc::nil();
        let mut e = e;
        loop {
            let (head, then, els) = match &e.kind {
                ExprKind::If(c, then, els) => (self.head(c), then, els),
                ExprKind::IfLet(c, then, els) => (self.let_cond(c), then, els),
                _ => {
                    doc = doc + self.block_or_expr(e);
                    break;
                }
            };
            doc = doc + "if " + head;
            doc = doc + self.block_or_expr(then);
            match els {
                None => break,
                Some(els) => {
                    doc = doc + " else ";
                    e = els;
                }
            }
        }
        group(doc)
    }

    /// The head of an `if` or `match`, and the space before its `{`. If it's
    /// a run of operators that has to break, the `{` goes on a line of its
    /// own, so the head doesn't run into the block:
    ///
    /// ```text
    /// if aaaa
    ///     || bbbb
    /// {
    /// ```
    fn head(&mut self, e: &Expr) -> Doc {
        match self.expr(
            e,
            0,
            Ctx {
                no_struct: true,
                stmt: false,
            },
        ) {
            Doc::Group { doc, .. } => group(*doc + Doc::Line),
            doc => doc + " ",
        }
    }

    /// `let pat = e ` after an `if` or `while`, with the space before the
    /// `{`. An `&&` or `||` there has to be in parentheses.
    fn let_cond(&mut self, c: &LetCond) -> Doc {
        let head = match chain_like(&c.expr) {
            true => text("(") + self.expr_inner(&c.expr, Ctx::default()) + ") ",
            false => self.head(&c.expr),
        };
        text("let ") + self.pattern(&c.pat) + " = " + head
    }

    /// The block of an `if` or `else`, ungrouped so that the `if` decides
    /// for all its blocks at once.
    fn block_or_expr(&mut self, e: &Expr) -> Doc {
        match &e.kind {
            ExprKind::Block(stmts, tail) => self.block(stmts, tail.as_deref(), e.span),
            _ => self.expr(e, 0, Ctx::default()),
        }
    }

    /// A block, which can go on one line if it's only an expression, and
    /// that isn't a block itself: `{ x + 1 }`, but not `{ let y = x; y }`
    /// or `{ if c { 1 } else { 2 } }`.
    fn block(&mut self, stmts: &[Stmt], tail: Option<&Expr>, span: Span) -> Doc {
        let close = span.end - 1;
        if stmts.is_empty() && tail.is_none() && !self.comments_before(close) {
            return text("{}");
        }
        let line = match (stmts, tail) {
            ([], Some(tail)) if !block_like(tail) => Doc::Line,
            _ => Doc::HardLine,
        };
        let mut body = self.trailing(span.start + 1);
        for (i, stmt) in stmts.iter().enumerate() {
            let (start, end) = match stmt {
                Stmt::Let {
                    pat, ty, init, els, ..
                } => {
                    let end = match (init, ty) {
                        _ if els.is_some() => els.as_ref().expect("there").span.end,
                        (Some(init), _) => init.span.end,
                        (None, Some(t)) => t.span.end,
                        (None, None) => pat.span.end,
                    };
                    (self.before(pat.span.start), end)
                }
                Stmt::Expr(e) | Stmt::BlockLike(e) => (e.span.start, e.span.end),
                Stmt::Fn(f) => (
                    lead_start(f.doc.as_ref(), &f.allows, f.span.start),
                    f.span.end,
                ),
                Stmt::Type(d) => {
                    let name = d.name().span;
                    let start = type_lead_start(d, self.before(name.start));
                    (start, self.find(name.end, &Tok::RBrace) + 1)
                }
            };
            body = body + line.clone() + self.element(start, i == 0);
            let doc = match stmt {
                Stmt::Let {
                    pat, ty, init, els, ..
                } => {
                    let ty = match ty {
                        Some(t) => text(": ") + self.ty(t),
                        None => Doc::nil(),
                    };
                    let head = text("let ") + self.pattern(pat) + ty;
                    match (init, els) {
                        (Some(init), Some(els)) => {
                            let init = match ends_in_brace(init) || chain_like(init) {
                                true => text("(") + self.expr_inner(init, Ctx::default()) + ")",
                                false => self.expr(init, 0, Ctx::default()),
                            };
                            group(head + " = " + init + " else " + self.block_or_expr(els)) + ";"
                        }
                        (Some(init), None) => {
                            head + " = " + self.expr(init, 0, Ctx::default()) + ";"
                        }
                        (None, _) => head + ";",
                    }
                }
                Stmt::Expr(e) => self.statement(e) + ";",
                Stmt::BlockLike(e) => self.expr(e, 0, Ctx::default()),
                Stmt::Fn(f) => self.fn_decl(f),
                Stmt::Type(d) => self.type_decl(d),
            };
            body = body + doc + self.trailing(end);
        }
        if let Some(tail) = tail {
            body = body + line.clone() + self.element(tail.span.start, stmts.is_empty());
            body = body + self.statement(tail) + self.trailing(tail.span.end);
        }
        for c in self.dangling(close, stmts.is_empty() && tail.is_none()) {
            body = body + Doc::HardLine + c;
        }
        text("{") + nest(body) + line + "}"
    }

    /// `e` at the start of a statement, where an `if`, `match` or block
    /// ends the statement.
    fn statement(&mut self, e: &Expr) -> Doc {
        self.expr_inner(
            e,
            Ctx {
                no_struct: false,
                stmt: true,
            },
        )
    }

    fn arm(&mut self, arm: &Arm) -> Doc {
        let pat = self.pattern(&arm.pat);
        pat + " => " + self.expr(&arm.body, 0, Ctx::default())
    }

    /// `::<i64, bool>` after a name, or nothing if there are no type
    /// arguments.
    fn turbofish(&mut self, targs: &[TypeExpr]) -> Doc {
        match targs.last() {
            None => Doc::nil(),
            Some(last) => {
                let close = self.find(last.span.end, &Tok::Gt);
                text("::") + self.types(ANGLES, close, targs)
            }
        }
    }

    fn types(&mut self, delims: Delims, close: usize, ts: &[TypeExpr]) -> Doc {
        self.list(
            delims,
            close,
            ts,
            |t| (t.span.start, t.span.end),
            None,
            |p, t| p.ty(t),
        )
    }

    fn ty(&mut self, t: &TypeExpr) -> Doc {
        match &t.kind {
            TypeExprKind::Refined(v, base, pred) => group(
                text("{ ")
                    + &v.name
                    + ": "
                    + self.ty(base)
                    + " | "
                    + self.expr(pred, 0, Ctx::default())
                    + " }",
            ),
            TypeExprKind::Named(name, args) if args.is_empty() => text(name),
            TypeExprKind::Named(name, args) => {
                let close = self.find(args.last().unwrap().span.end, &Tok::Gt);
                text(name) + self.types(ANGLES, close, args)
            }
            TypeExprKind::Unit => text("()"),
            TypeExprKind::Never => text("!"),
            TypeExprKind::Lifetime(name) => text(name),
            TypeExprKind::Ref(lifetime, mutable, inner) => {
                let mut head = String::from("&");
                if let Some(l) = lifetime {
                    head = head + &l.name + " ";
                }
                if *mutable {
                    head += "mut ";
                }
                text(head) + self.ty(inner)
            }
            TypeExprKind::Slice(inner) => text("[") + self.ty(inner) + "]",
            TypeExprKind::Array(inner, n) => text("[") + self.ty(inner) + "; " + self.ty(n) + "]",
            TypeExprKind::Nat(n) => self.spelled(t.span, n.to_string()),
            TypeExprKind::Proj(inner, Some(tr), name) => {
                text("<") + self.ty(inner) + " as " + &tr.name + ">::" + &name.name
            }
            TypeExprKind::Proj(inner, None, name) => self.ty(inner) + "::" + &name.name,
            TypeExprKind::Impl(bounds) => {
                let mut out = text("impl ");
                for (i, b) in bounds.iter().enumerate() {
                    if i > 0 {
                        out = out + " + ";
                    }
                    out = out + self.bound(b);
                }
                out
            }
            TypeExprKind::Fn(params, ret, effects) => {
                let open = self.find(t.span.start, &Tok::LParen);
                let close = self.close(open, params.last().map(|p| p.span.end), &Tok::RParen);
                let params = self.types(PARENS, close, params);
                let out = match ret.kind {
                    TypeExprKind::Unit => text("fn") + params,
                    _ => text("fn") + params + " -> " + self.ty(ret),
                };
                match effects {
                    Some(e) => out + " " + self.effects(e),
                    None => out,
                }
            }
        }
    }

    fn pattern(&mut self, p: &Pattern) -> Doc {
        match &p.kind {
            // As for `ExprKind::Move`, there's no syntax for one.
            PatternKind::Move(p) => self.pattern(p),
            PatternKind::Wild => text("_"),
            PatternKind::Bind(name, false) => text(name),
            PatternKind::Bind(name, true) => text(format!("mut {name}")),
            PatternKind::Int(n, lit) => self.spelled(p.span, lit.show(*n)),
            PatternKind::Range(lo, hi, inclusive, lit) => {
                self.spelled(p.span, range(*lo, *hi, *inclusive, *lit))
            }
            PatternKind::Bool(b) => text(b.to_string()),
            PatternKind::Unit => text("()"),
            PatternKind::Variant(ty, targs, variant, args) => {
                let head = text(&ty.name) + self.turbofish(targs) + "::" + &variant.name;
                if args.is_empty() {
                    return head;
                }
                let close = p.span.end - 1;
                head + self.list(
                    PARENS,
                    close,
                    args,
                    |a| (a.span.start, a.span.end),
                    None,
                    |p, a| p.pattern(a),
                )
            }
            PatternKind::Struct(ty, targs, fields, rest) => {
                let close = p.span.end - 1;
                let rest = rest.then(|| {
                    let from = fields.last().map_or(ty.span.end, |f| f.pat.span.end);
                    (self.find(from, &Tok::DotDot), "..")
                });
                let fields = self.list(
                    BRACES,
                    close,
                    fields,
                    |f| (f.name.span.start, f.pat.span.end),
                    rest,
                    |p, f| match &f.pat.kind {
                        PatternKind::Bind(b, false) if *b == f.name.name => text(b),
                        _ => text(&f.name.name) + ": " + p.pattern(&f.pat),
                    },
                );
                text(&ty.name) + self.turbofish(targs) + " " + fields
            }
            PatternKind::Array(pats) => self.list(
                BRACKETS,
                p.span.end - 1,
                pats,
                |a| (a.span.start, a.span.end),
                None,
                |p, a| p.pattern(a),
            ),
            // Not indented when it breaks, so that in a `match` the body
            // stands out from the alternatives.
            PatternKind::Or(alts) => {
                let mut doc = self.pattern(&alts[0]);
                for alt in &alts[1..] {
                    doc = doc + Doc::Line + "| " + self.pattern(alt);
                }
                group(doc)
            }
        }
    }
}

/// Where a function parameter starts: at its `mut`, if it has one.
fn param_start(p: &Param) -> usize {
    p.mutable.unwrap_or(p.name.span).start.min(p.ty.span.start)
}

/// Where something with the doc comment `doc` and attributes `allows`
/// starts, if it starts at `pos` without them.
fn lead_start(doc: Option<&DocComment>, allows: &[Allow], pos: usize) -> usize {
    let doc = doc.map(|d| d.span().start);
    let allow = allows.first().map(|a| a.span.start);
    [doc, allow].into_iter().flatten().fold(pos, usize::min)
}

/// Where struct or enum `d` starts, if it starts at `pos` without its doc
/// comment and attributes.
fn type_lead_start(d: &TypeDecl, pos: usize) -> usize {
    let derive = d.derives().first().map_or(pos, |d| d.span.start);
    let intrinsic = match d {
        TypeDecl::Struct(StructDecl {
            intrinsic: Some(at),
            ..
        }) => at.start,
        _ => pos,
    };
    lead_start(d.doc(), d.allows(), pos)
        .min(derive)
        .min(intrinsic)
}

/// A range pattern as written. Unlike `exhaust::show_range`, which tidies
/// ranges up for messages, this keeps `0..1` from turning into `0`.
fn range(lo: Option<i128>, hi: Option<i128>, inclusive: bool, lit: Lit) -> String {
    let lo = lo.map_or(String::new(), |lo| lit.show(lo));
    match hi {
        None => format!("{lo}.."),
        Some(hi) if inclusive => format!("{lo}..={}", lit.show(hi)),
        Some(hi) => format!("{lo}..{}", lit.show(hi + 1)),
    }
}

/// Something in the `{ .. }` of a trait or impl.
enum BodyItem<'a> {
    /// A method, with whether it has a body.
    Fn(&'a FnDecl, bool),
    /// `type Item: Discard;`, in a trait.
    Decl(&'a AssocDecl),
    /// `type Item = i64;`, in an impl.
    Def(&'a AssocDef),
    /// `effect e;`, in a trait.
    EffectDecl(&'a EffectDecl),
    /// `effect e = panic;`, in an impl.
    EffectDef(&'a EffectDef),
}

impl BodyItem<'_> {
    fn span(&self) -> Span {
        match self {
            BodyItem::Fn(f, _) => lead_start_span(f),
            BodyItem::Decl(a) => a.span,
            BodyItem::Def(a) => a.span,
            BodyItem::EffectDecl(a) => a.span,
            BodyItem::EffectDef(a) => a.span,
        }
    }
}

/// Where a method starts, docs and attributes and all.
fn lead_start_span(f: &FnDecl) -> Span {
    Span::new(
        lead_start(f.doc.as_ref(), &f.allows, fn_start(f)),
        f.span.end,
    )
}

/// Where function `f` starts without its doc comment and attributes: at its
/// `pub`, if it has one.
fn fn_start(f: &FnDecl) -> usize {
    let start = f.vis.span().map_or(f.span.start, |p| p.start);
    let start = f.intrinsic.map_or(start, |i| i.start.min(start));
    let start = f.measure.map_or(start, |i| i.start.min(start));
    f.test.map_or(start, |t| t.span.start.min(start))
}

/// `pub ` or `export `, as `v` is.
fn vis(v: Vis) -> Doc {
    match v {
        Vis::Private => Doc::nil(),
        v => text(v.keyword()),
    }
}

/// `a::{b, c as d}` in a `use`.
fn use_tree(t: &UseTree) -> String {
    let path: Vec<&str> = t.path.iter().map(|s| s.name.as_str()).collect();
    let path = path.join("::");
    match &t.kind {
        UseKind::Name(None) => path,
        UseKind::Name(Some(alias)) => format!("{path} as {}", alias.name),
        UseKind::Group(trees) => {
            let trees: Vec<String> = trees.iter().map(use_tree).collect();
            match path.is_empty() {
                true => format!("{{{}}}", trees.join(", ")),
                false => format!("{path}::{{{}}}", trees.join(", ")),
            }
        }
    }
}

/// `where T: Discard, U: Eq` after a derived trait.
fn where_clause(w: &Where) -> String {
    let bounds: Vec<String> = w
        .bounds
        .iter()
        .map(|(p, items)| {
            let items: Vec<String> = items.iter().map(BoundItem::to_string).collect();
            format!("{}: {}", p.name, items.join(" + "))
        })
        .collect();
    format!("where {}", bounds.join(", "))
}

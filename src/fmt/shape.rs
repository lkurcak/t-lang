//! A structural description of a parsed AST, without source positions or
//! node numbers. Text is escaped as text, never searched for metadata.
//!
//! Every match is exhaustive, including struct fields: adding syntax to
//! the AST must also decide how the formatter compares it.

use crate::ast::*;
use crate::error::Span;
use crate::types::IntTy;
use std::fmt::{self, Debug};
use std::sync::Arc;

pub(super) fn of(program: &Program) -> String {
    format!("{:?}", Shape(program))
}

struct Shape<'a, T: ?Sized>(&'a T);

trait Describe {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T: Describe + ?Sized> Debug for Shape<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.describe(f)
    }
}

macro_rules! leaves {
    ($($ty:ty),* $(,)?) => {$(
        impl Describe for $ty {
            fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                Debug::fmt(self, f)
            }
        }
    )*};
}

leaves!(
    String, bool, char, u8, u64, i128, usize, IntTy, UnOp, BinOp, Lit, Capture
);

impl Describe for Span {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_")
    }
}

impl Describe for NodeId {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_")
    }
}

impl<T: Describe> Describe for Vec<T> {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter().map(Shape)).finish()
    }
}

impl<T: Describe> Describe for Option<T> {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Some(x) => f.debug_tuple("Some").field(&Shape(x)).finish(),
            None => f.write_str("None"),
        }
    }
}

macro_rules! pointers {
    ($($ty:ty),* $(,)?) => {$(
        impl<T: Describe> Describe for $ty {
            fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (**self).describe(f)
            }
        }
    )*};
}

pointers!(&T, Box<T>, Arc<T>);

macro_rules! tuples {
    ($($($field:ident),+);+ $(;)?) => {$(
        impl<$($field: Describe),+> Describe for ($($field,)+) {
            fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                #[allow(non_snake_case)]
                let ($($field,)+) = self;
                f.debug_tuple("")$(.field(&Shape($field)))+.finish()
            }
        }
    )+};
}

tuples!(A, B; A, B, C);

macro_rules! structs {
    ($($ty:ident { $($field:ident),* $(,)? });* $(;)?) => {$(
        impl Describe for $ty {
            fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self { $($field),* } = self;
                f.debug_struct(stringify!($ty))
                    $(.field(stringify!($field), &Shape($field)))*
                    .finish()
            }
        }
    )*};
}

structs! {
    DocComment { lines, inner };
    DocLine { text, span };
    Allow { span, lints };
    ModDecl { doc, allows, span, name, parent, vis, in_file };
    UseDecl { span, module, vis, tree };
    UseTree { path, kind, span };
    BoundItem { name, assoc, sig, effects, span };
    AssocBound { name, kind };
    Derive { span, names, wheres };
    Where { span, bounds };
    StructDecl { doc, allows, span, module, vis, name, generics, derives, fields, intrinsic };
    EnumDecl { doc, allows, span, module, vis, name, generics, derives, variants };
    AliasDecl { doc, allows, span, module, vis, name, generics, ty };
    Variant { doc, allows, span, name, fields };
    FnDecl { doc, allows, span, module, vis, name, generics, params, parens, ret,
        effects, body, intrinsic, measure, ensures };
    TraitDecl { doc, allows, intrinsic, sealed, span, module, vis, name, supers, assoc, effects, methods };
    AssocDecl { doc, span, name, bounds };
    AssocDef { doc, span, name, ty };
    EffectDecl { doc, span, name };
    EffectDef { doc, span, name, effects };
    EffectSet { items, span, braced, with };
    Effect { kind, span, id };
    TraitMethod { decl, provided };
    ImplDecl { doc, allows, span, module, generics, trait_name, self_ty, assoc, effects, methods };
    Param { doc, allows, mutable, vis, name, ty };
    Ident { name, span, id };
    TypeExpr { kind, span, id };
    Expr { kind, span, id };
    ForLoop { label, pat, in_span, iter, body };
    Label { name, head };
    LetCond { let_span, pat, expr };
    Lambda { id, moves, params, ret, body, captures, modes };
    LambdaParam { mutable, name, ty };
    FieldInit { name, value };
    Arm { pat, body };
    Pattern { kind, span, id };
    FieldPat { name, pat };
}

macro_rules! enums {
    ($($ty:ident { $($variant:ident $(($($field:ident),+))?),* $(,)? });* $(;)?) => {$(
        impl Describe for $ty {
            fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$variant $(($($field),+))? => {
                        f.debug_tuple(stringify!($variant))
                            $($(.field(&Shape($field)))+)?
                            .finish()
                    }),*
                }
            }
        }
    )*};
}

enums! {
    Vis { Private, Pub(span), Export(span) };
    UseKind { Name(alias), Group(trees) };
    TypeDecl { Struct(decl), Enum(decl), Alias(decl) };
    AssocBoundKind { Eq(ty), Bounds(bounds) };
    EffectKind { Named(name), Proj(ty, tr, name) };
    TypeExprKind {
        Named(name, args), Unit, Fn(params, ret, effects), Ref(lifetime, mutable, ty),
        Slice(ty), Array(ty, len), Nat(n), Lifetime(name), Never, Proj(ty, tr, name),
        Impl(bounds), Refined(name, ty, pred),
    };
    ExprKind {
        Int(n, ty), Bool(b), Char(c), Str(s), ByteStr(bytes), Unit, Var(name, args),
        Unary(op, expr), As(expr, ty), Binary(op, left, right), Call(callee, args),
        MethodCall(receiver, name, types, args), If(cond, then, otherwise),
        IfLet(cond, then, otherwise), Array(elems),
        Repeat(value, len), Block(stmts, tail), Unsafe(block), StructLit(name, types, fields),
        Field(expr, name), Variant(ty, types, name, args, called), Match(expr, arms), Lambda(lambda),
        Assign(op, place, value), While(label, cond, body), WhileLet(label, cond, body),
        Loop(label, body), For(loop_),
        Range(start, end), Index(expr, index, span, mutable), Try(expr, span), Return(expr),
        Break(label, expr), Continue(label), Move(expr),
    };
    PatternKind {
        Wild, Bind(name, mutable), Int(n, kind), Range(start, end, inclusive, kind), Bool(b), Unit,
        Variant(ty, types, name, fields), Struct(ty, types, fields, rest), Array(elems), Or(alts),
        Move(pat),
    };
}

impl Describe for Stmt {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Let {
                pat,
                ty,
                init,
                els,
                span,
            } => f
                .debug_struct("Let")
                .field("pat", &Shape(pat))
                .field("ty", &Shape(ty))
                .field("init", &Shape(init))
                .field("els", &Shape(els))
                .field("span", &Shape(span))
                .finish(),
            Self::Expr(expr) => f.debug_tuple("Expr").field(&Shape(expr)).finish(),
            Self::BlockLike(expr) => f.debug_tuple("BlockLike").field(&Shape(expr)).finish(),
            Self::Fn(decl) => f.debug_tuple("Fn").field(&Shape(decl)).finish(),
            Self::Type(decl) => f.debug_tuple("Type").field(&Shape(decl)).finish(),
        }
    }
}

impl Describe for Generics {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            lifetimes,
            types,
            effects,
            consts,
            bounds,
            span,
            wheres,
            where_span,
        } = self;
        // These ids refer to parameters: compare which parameter each
        // names, not its source node number (nor just the number of ids).
        let parameter = |id: &NodeId| types.iter().position(|p| p.id == *id);
        let effects: Vec<_> = effects.iter().map(parameter).collect();
        let consts: Vec<_> = consts.iter().map(|(id, ty)| (parameter(id), ty)).collect();
        f.debug_struct("Generics")
            .field("lifetimes", &Shape(lifetimes))
            .field("types", &Shape(types))
            .field("effects", &Shape(&effects))
            .field("consts", &Shape(&consts))
            .field("bounds", &Shape(bounds))
            .field("span", &Shape(span))
            .field("wheres", &Shape(wheres))
            .field("where_span", &Shape(where_span))
            .finish()
    }
}

impl Describe for Program {
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            mods,
            core,
            libraries,
            packages: _,
            uses,
            types,
            traits,
            impls,
            fns,
            stray_docs,
            next_id: _,
            proved: _,
            measures: _,
        } = self;
        // A parsed program has no analysis results, nor packages, which
        // loading finds. The next node number
        // isn't syntax either. Stray docs are compared line by line:
        // formatting can regroup them while keeping each unattached.
        let stray_docs: Vec<_> = stray_docs
            .iter()
            .flat_map(|d| d.lines.iter().map(move |line| (d.inner, line)))
            .collect();
        f.debug_struct("Program")
            .field("mods", &Shape(mods))
            .field("core", &Shape(core))
            .field("libraries", &Shape(libraries))
            .field("uses", &Shape(uses))
            .field("types", &Shape(types))
            .field("traits", &Shape(traits))
            .field("impls", &Shape(impls))
            .field("fns", &Shape(fns))
            .field("stray_docs", &Shape(&stray_docs))
            .finish()
    }
}

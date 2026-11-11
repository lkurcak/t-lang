//! Types written out: where a type alias with a refinement in it is used,
//! or one given a refined argument, what the refinement checker reads is
//! the type with the alias written out by hand (see
//! docs/67-refinement-aliases.md, and lemma A1 of docs/25-type-aliases.md).
//!
//! What a type *is* doesn't need this: resolving a use of an alias resolves
//! what it stands for, with its refinements left out (`generics::At`).
//! Refinements aren't in `Type`; they're read off types as written
//! (`TypeExpr::refinements`), each at a position of the written type. So
//! the checker reads them off a copy (`Checker::written_out`), where each
//! such alias is what it stands for, with each of its parameters replaced by
//! the argument written for it, wherever it is in that, and each predicate a
//! copy of its own, of new nodes: what the checker learns of a predicate
//! (which value each name is, which ways into a value it's of) it keeps by
//! node, and that's different at each use, and for each place a parameter
//! is used. Two uses never share a predicate's node.
//!
//! The names in what an alias stands for mean what they do where it's
//! declared (R2). A type's are looked up by node, and a copy keeps the
//! nodes of the types in it, so they do. A predicate's are looked up by
//! name: one copied from an alias (*alias-origin*, `Origin`) is checked in
//! the alias's module, naming only its binder and the consts there, never
//! what's where it's used; and what the alias's const parameters stand for
//! (an argument, from where the alias is used) is put in by what it
//! resolved to there (`Fixed`), so no name in the alias can catch it.

use super::Checker;
use super::names::Scoped;
use crate::ast::*;
use crate::check::AliasDef;
use std::collections::HashMap;

/// Where a predicate written out of an alias is from: the alias, whose
/// module its names mean what they do in, and which is seen as far as its
/// `vis` says.
#[derive(Debug, Clone)]
pub(crate) struct Origin {
    pub(crate) alias: String,
    pub(crate) module: usize,
    pub(crate) vis: Vis,
}

/// What a name in a predicate copied from an alias stands for, if it's
/// what was given for one of the alias's const parameters, where the alias
/// was used.
#[derive(Debug, Clone)]
pub(crate) enum Fixed {
    /// A const parameter there, by name, written at this span.
    Param(String, crate::error::Span),
    /// A const item, by symbol.
    Item(String),
}

/// What an alias's const parameter was given, in a use of it.
#[derive(Debug, Clone)]
enum ConstArg {
    Lit(u64),
    Param(String, crate::error::Span),
    /// A const item, by symbol and name.
    Item(String, String),
}

/// The alias being written out, at one use of it.
struct Use<'a> {
    def: &'a AliasDef,
    origin: Origin,
    /// What each type parameter stands for: its argument, written out.
    types: HashMap<String, TypeExpr>,
    consts: HashMap<String, ConstArg>,
}

impl Checker {
    /// `t`, a type written where a refinement can be, as the refinement
    /// checker reads it: with every alias written out that's a refinement
    /// alias, or is given a refined argument (see the module's comment).
    /// The same copy each time it's asked for `t`.
    pub(super) fn written_out(&mut self, t: &TypeExpr) -> TypeExpr {
        if let Some(w) = self.info.written.get(&t.id) {
            return w.clone();
        }
        if !self.to_write_out(t, true) {
            return t.clone();
        }
        let mut w = self.write_out(t, None);
        // Its root is `t`'s node, not that of what an alias stands for:
        // asked again for it, this is what it gets.
        w.id = t.id;
        self.info.written.insert(t.id, w.clone());
        w
    }

    /// The alias that type `t` names, if it's a use of one.
    fn alias_of(&self, t: &TypeExpr) -> Option<std::sync::Arc<AliasDef>> {
        match self.type_names.get(&t.id)? {
            Scoped::Adt(symbol) => self.aliases.get(symbol).cloned(),
            Scoped::OuterParam(..) => None,
        }
    }

    /// Is there anything to write out in `t`, where a refinement can be if
    /// `ok`: a refinement alias, or an alias with a refinement in an
    /// argument, where one can be?
    fn to_write_out(&self, t: &TypeExpr, ok: bool) -> bool {
        if !ok {
            return false;
        }
        match &t.kind {
            TypeExprKind::Refined(_, base, _) => self.to_write_out(base, ok),
            TypeExprKind::Named(_, args) => match self.alias_of(t) {
                Some(def) => {
                    def.refined
                        || ty_args(args).enumerate().any(|(k, a)| {
                            def.refinable.get(k) == Some(&true)
                                && (!a.refinements().is_empty() || self.to_write_out(a, true))
                        })
                }
                None => ty_args(args).any(|a| self.to_write_out(a, true)),
            },
            TypeExprKind::Tuple(ts) => ts.iter().any(|e| self.to_write_out(e, ok)),
            TypeExprKind::Ref(_, _, inner)
            | TypeExprKind::Slice(inner)
            | TypeExprKind::Array(inner, _) => self.to_write_out(inner, ok),
            _ => false,
        }
    }

    /// `t` written out, in what alias use `at` stands for if it's in one
    /// (or else where it was written).
    fn write_out(&mut self, t: &TypeExpr, at: Option<&Use>) -> TypeExpr {
        let kind = match &t.kind {
            TypeExprKind::Refined(v, base, pred) => {
                let base = self.write_out(base, at);
                let pred = match at {
                    Some(at) => self.copy_pred(pred, at),
                    None => (**pred).clone(),
                };
                TypeExprKind::Refined(v.clone(), Box::new(base), Box::new(pred))
            }
            TypeExprKind::Named(n, args)
                if args.is_empty()
                    && let Some(arg) = at.and_then(|at| at.types.get(n)) =>
            {
                let arg = arg.clone();
                return self.freshen(&arg);
            }
            TypeExprKind::Named(n, args) => {
                if let Some(def) = self.alias_of(t)
                    && (at.is_some() || self.to_write_out(t, true))
                {
                    return self.expand_alias(&def, args, at);
                }
                let args = (args.iter())
                    .map(|a| match a.kind {
                        TypeExprKind::Lifetime(_) => a.clone(),
                        _ => self.write_out(a, at),
                    })
                    .collect();
                TypeExprKind::Named(n.clone(), args)
            }
            TypeExprKind::Tuple(ts) => {
                TypeExprKind::Tuple(ts.iter().map(|e| self.write_out(e, at)).collect())
            }
            TypeExprKind::Ref(l, m, inner) => {
                TypeExprKind::Ref(l.clone(), *m, Box::new(self.write_out(inner, at)))
            }
            TypeExprKind::Slice(inner) => TypeExprKind::Slice(Box::new(self.write_out(inner, at))),
            TypeExprKind::Array(inner, n) => {
                TypeExprKind::Array(Box::new(self.write_out(inner, at)), n.clone())
            }
            k => k.clone(),
        };
        TypeExpr {
            kind,
            span: t.span,
            id: t.id,
        }
    }

    /// What a use of alias `def` with arguments `args`, written in alias
    /// use `at` if it's in one, stands for, written out.
    fn expand_alias(&mut self, def: &AliasDef, args: &[TypeExpr], at: Option<&Use>) -> TypeExpr {
        let mut types = HashMap::new();
        let mut consts = HashMap::new();
        for (k, a) in ty_args(args).enumerate() {
            let (Some(p), Some(kind)) = (def.params.get(k), def.kinds.get(k)) else {
                continue;
            };
            match kind {
                ArgKind::Length => {
                    consts.insert(p.clone(), self.const_arg(a, at));
                }
                _ if def.refinable.get(k) == Some(&true) => {
                    types.insert(p.clone(), self.write_out(a, at));
                }
                _ => {
                    types.insert(p.clone(), a.clone());
                }
            }
        }
        let origin = Origin {
            alias: def.name.clone(),
            module: def.module,
            vis: def.vis,
        };
        let here = Use {
            def,
            origin,
            types,
            consts,
        };
        let body = here.def.body.clone();
        self.write_out(&body, Some(&here))
    }

    /// What length `a`, an argument for an alias's const parameter written
    /// in alias use `at` if it's in one, is.
    fn const_arg(&self, a: &TypeExpr, at: Option<&Use>) -> ConstArg {
        match &a.kind {
            TypeExprKind::Nat(n) => ConstArg::Lit(*n),
            TypeExprKind::Named(m, args) if args.is_empty() => {
                if let Some(c) = at.and_then(|at| at.consts.get(m)) {
                    return c.clone();
                }
                match self.modules.lengths.get(&a.id) {
                    Some(&(i, _)) => ConstArg::Item(self.fn_symbols[i].clone(), m.clone()),
                    None => ConstArg::Param(m.clone(), a.span),
                }
            }
            // Resolving the use found anything else wrong already.
            _ => ConstArg::Lit(0),
        }
    }

    /// A copy of predicate `pred`, of what alias use `at` stands for, of
    /// new nodes: each const parameter's name replaced by what it was
    /// given.
    fn copy_pred(&mut self, pred: &Expr, at: &Use) -> Expr {
        let mut copy = pred.clone();
        self.renew(&mut copy, Some(at));
        self.alias_preds.insert(copy.id, at.origin.clone());
        copy
    }

    /// `t`, with every predicate in it copied, of new nodes, as what it was:
    /// for a parameter of an alias, written out where it's used, which may
    /// be in more than one place.
    fn freshen(&mut self, t: &TypeExpr) -> TypeExpr {
        let kind = match &t.kind {
            TypeExprKind::Refined(v, base, pred) => {
                let base = self.freshen(base);
                let mut copy = (**pred).clone();
                self.renew(&mut copy, None);
                if let Some(o) = self.alias_preds.get(&pred.id).cloned() {
                    self.alias_preds.insert(copy.id, o);
                }
                TypeExprKind::Refined(v.clone(), Box::new(base), Box::new(copy))
            }
            TypeExprKind::Named(n, args) => {
                TypeExprKind::Named(n.clone(), args.iter().map(|a| self.freshen(a)).collect())
            }
            TypeExprKind::Tuple(ts) => {
                TypeExprKind::Tuple(ts.iter().map(|e| self.freshen(e)).collect())
            }
            TypeExprKind::Ref(l, m, inner) => {
                TypeExprKind::Ref(l.clone(), *m, Box::new(self.freshen(inner)))
            }
            TypeExprKind::Slice(inner) => TypeExprKind::Slice(Box::new(self.freshen(inner))),
            TypeExprKind::Array(inner, n) => {
                TypeExprKind::Array(Box::new(self.freshen(inner)), n.clone())
            }
            k => k.clone(),
        };
        TypeExpr {
            kind,
            span: t.span,
            id: t.id,
        }
    }

    /// Gives every node of `e` a new id, remembering which it was a copy
    /// of; and if it's of what alias use `at` stands for, puts what its
    /// const parameters were given for their names.
    fn renew(&mut self, e: &mut Expr, at: Option<&Use>) {
        let old = e.id;
        let new = self.fresh_id();
        e.id = new;
        let original = self.copied.get(&old).copied().unwrap_or(old);
        self.copied.insert(new, original);
        if let Some(f) = self.pred_fixed.get(&old).cloned() {
            self.pred_fixed.insert(new, f);
        }
        if let Some(at) = at
            && let ExprKind::Var(name, targs) = &e.kind
            && targs.is_empty()
            && let Some(c) = at.consts.get(name).cloned()
        {
            match c {
                ConstArg::Lit(n) => e.kind = ExprKind::Int(n, None),
                ConstArg::Param(m, span) => {
                    e.kind = ExprKind::Var(m.clone(), Vec::new());
                    self.pred_fixed.insert(new, Fixed::Param(m, span));
                }
                ConstArg::Item(symbol, m) => {
                    e.kind = ExprKind::Var(m, Vec::new());
                    self.pred_fixed.insert(new, Fixed::Item(symbol));
                }
            }
            return;
        }
        for c in pred_children(e) {
            self.renew(c, at);
        }
    }
}

/// The expressions directly inside `e`, a predicate: of the kinds the
/// logic has (`Checker::pred_type`). Any other is an error there.
fn pred_children(e: &mut Expr) -> Vec<&mut Expr> {
    match &mut e.kind {
        ExprKind::Unary(_, x) | ExprKind::As(x, _) | ExprKind::Field(x, _) => vec![&mut **x],
        ExprKind::Binary(_, l, r) | ExprKind::Index(l, r, ..) | ExprKind::Range(l, r) => {
            vec![&mut **l, &mut **r]
        }
        ExprKind::Call(f, args) | ExprKind::MethodCall(f, _, _, args) => {
            std::iter::once(&mut **f).chain(args.iter_mut()).collect()
        }
        ExprKind::If(c, then, els) => [&mut **c, &mut **then]
            .into_iter()
            .chain(els.as_deref_mut())
            .collect(),
        ExprKind::Tuple(args) | ExprKind::Array(args) => args.iter_mut().collect(),
        _ => Vec::new(),
    }
}

/// The type and const arguments in `args`, without the lifetimes.
fn ty_args(args: &[TypeExpr]) -> impl Iterator<Item = &TypeExpr> {
    args.iter()
        .filter(|a| !matches!(a.kind, TypeExprKind::Lifetime(_)))
}

#[cfg(test)]
mod tests {
    use crate::ast::NodeId;

    /// The predicates the checker read of each written type that had an
    /// alias written out, in order.
    fn preds(src: &str) -> (Vec<Vec<NodeId>>, std::sync::Arc<crate::check::Info>) {
        let checked = crate::check_source(src);
        assert!(checked.result.is_ok(), "{src}");
        let mut written: Vec<_> = checked.info.written.iter().collect();
        written.sort_by_key(|(id, _)| **id);
        let out = (written.iter())
            .map(|(_, t)| t.refinements().iter().map(|(_, _, p)| p.id).collect())
            .collect();
        (out, checked.info.clone())
    }

    /// Each place a parameter is, in what an alias stands for, gets a copy
    /// of the predicate given for it of its own, as does each use: what's
    /// kept by a predicate's node, such as which values of a type the
    /// interpreter checks it of, is that place's (lemma A5 of
    /// docs/67-refinement-aliases.md).
    #[test]
    fn each_place_and_each_use_has_its_own_predicate() {
        let src = "type Pair<T> = (T, T);\n\
                   type Square = { s: u8 | s < 64 };\n\
                   fn g(_p: Pair<{ v: u8 | v < 3 }>, _a: Square, _b: Square) {}\n\
                   fn main() { g((1, 2), 3, 4); }\n";
        let (preds, info) = preds(src);
        let all: Vec<NodeId> = preds.iter().flatten().copied().collect();
        assert_eq!(all.len(), 4, "{preds:?}");
        let mut distinct = all.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 4, "{preds:?}");
        let pair = preds.iter().find(|p| p.len() == 2).expect("the pair's");
        assert_ne!(
            info.measures.reach.get(&pair[0]),
            info.measures.reach.get(&pair[1])
        );
    }
}

//! Effect inference (see docs/24-visibility.md): a function that isn't
//! exported and doesn't say `with` has the effects its body has, rather
//! than none. What's exported says its effects, as other packages count
//! on them; inside a package, whatever counts on a function's effects is
//! checked again whenever they change.
//!
//! Until its body is checked, such a function's effects are a placeholder,
//! `Type::Inferred`: what will have been inferred for it, at its type
//! arguments. Bodies are checked callees first (`body_groups`), so a call
//! of one already checked has its effects as if they were written, and
//! only a call within a recursive group, of a function whose body isn't
//! done yet, has a placeholder. Once all of a group's bodies are, its
//! effects are the least that hold for all of them (`solve_group`), and
//! what had to wait for them is checked: the `Within`s whose sets had a
//! placeholder, which `settle_effects` kept (`Deferred`).
//!
//! Soundness: the solution has, for each inferred function, every effect
//! of every site in its body, a call's being its callee's at its arguments,
//! so step 7's lemma (docs/07-effects.md) goes through with "the effects it
//! says" read as "says or was inferred to have": a function that has no
//! `panic` reaches none. A least one exists, and is found by going round
//! until nothing changes: each step only adds effects, each function's are
//! drawn from finitely many (`alloc`, `panic`, its own effect parameters
//! and its parameters' associated effects), and a set's union with more is
//! monotone. What's checked against it is checked at the least, which an
//! upper bound holds at if it holds at any. `unsafe` is never inferred: it
//! says its caller keeps obligations, which a body can't promise.
//!
//! No placeholder is left once the package is checked (`zonk_inferred`):
//! what's kept of it, and what monomorphising reads, has its functions'
//! effects as they are.

use super::Checker;
use super::Instance;
use super::effects::Site;
use super::scope::FnId;
use crate::ast::*;
use crate::error::{Error, Span};
use crate::types::{Subst, Type};
use std::collections::{HashMap, HashSet};

/// More effects than any function's inferred ones can have, but by
/// calling itself at bigger and bigger types (see `solve_group`).
const MOST_EFFECTS: usize = 256;

/// Does `f`, a method of `method` if it's one, have the effects its body
/// has, rather than those it says? If it isn't exported, says no `with`,
/// and its effects are its own: not a trait's method, whose effects its
/// trait says for every impl, nor an intrinsic or a measure, which have
/// no body to infer them from, or must have none.
pub(super) fn infers(prog: &Program, f: &FnDecl, method: Option<MethodOf>) -> bool {
    f.effects.is_none()
        && !f.vis.is_export()
        && f.intrinsic.is_none()
        && f.measure.is_none()
        && match method {
            None => true,
            Some(MethodOf::Impl(k)) => prog.impls[k].trait_name.is_none(),
            Some(MethodOf::Trait(_)) => false,
        }
}

/// The function at `i` in `fns` (as `Program::all_fns`), or the one it's
/// nested in, all the way out.
pub(super) fn root(fns: &[(&FnDecl, Option<usize>, Option<MethodOf>)], mut i: usize) -> usize {
    while let Some(p) = fns[i].1 {
        i = p;
    }
    i
}

/// The order to check the bodies of the package's functions in, those in
/// `fns` from `from` on (as `Program::all_fns`): groups of top-level ones,
/// each with the ones nested in them, callees first. Each group is those
/// that may call each other: a strongly connected component of the graph
/// of which may call which, of which the groups after it are never
/// callees. Measures come first, as `bodies` needs.
///
/// Which may call which is only what the names in their bodies suggest:
/// each name of a function, method or variant there is taken to be a call
/// of every inferred function with that name. That's more than calls, so
/// a group may be bigger than it needs to be, which only makes more wait
/// for it: it's never less, which would leave a callee's effects unknown
/// after its group (which `solve_group` copes with too, by waiting).
pub(super) fn body_groups(
    prog: &Program,
    fns: &[(&FnDecl, Option<usize>, Option<MethodOf>)],
    from: usize,
) -> Vec<Vec<usize>> {
    let own: Vec<usize> = (from..fns.len()).collect();
    let mut by_name: HashMap<&str, HashSet<usize>> = HashMap::new();
    for &i in &own {
        let (f, _, method) = fns[i];
        if infers(prog, f, method) {
            by_name
                .entry(&f.name.name)
                .or_default()
                .insert(root(fns, i));
        }
    }
    let tops: Vec<usize> = own
        .iter()
        .copied()
        .filter(|&i| fns[i].1.is_none())
        .collect();
    let index: HashMap<usize, usize> = tops.iter().enumerate().map(|(k, &i)| (i, k)).collect();
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); tops.len()];
    for &i in &own {
        let mut names = HashSet::new();
        mentioned(&fns[i].0.body, &mut names);
        let from = index[&root(fns, i)];
        for name in names {
            for &j in by_name.get(name.as_str()).into_iter().flatten() {
                edges[from].push(index[&j]);
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = sccs(&edges)
        .into_iter()
        .map(|g| g.into_iter().map(|k| tops[k]).collect())
        .collect();
    groups.sort_by_key(|g: &Vec<usize>| !g.iter().any(|&i| fns[i].0.measure.is_some()));
    groups
}

/// The last segment of each name in `e` that may name a function or
/// method: variables, methods called, and `T::m` paths. Not those in the
/// functions nested in it, which have bodies of their own.
fn mentioned(e: &Expr, out: &mut HashSet<String>) {
    let last = |n: &str| n.rsplit("::").next().unwrap_or(n).to_string();
    match &e.kind {
        ExprKind::Var(n, _) => {
            out.insert(last(n));
        }
        ExprKind::MethodCall(_, name, ..) | ExprKind::Variant(_, _, name, ..) => {
            out.insert(name.name.clone());
        }
        _ => {}
    }
    for c in e.children() {
        mentioned(c, out);
    }
}

/// The strongly connected components of the graph whose edges from each
/// node are `edges`, each after every component it has an edge to
/// (Tarjan's algorithm, which finds them in that order).
fn sccs(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct State<'e> {
        edges: &'e [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn visit(s: &mut State, v: usize) {
        s.index[v] = Some(s.next);
        s.low[v] = s.next;
        s.next += 1;
        s.stack.push(v);
        s.on_stack[v] = true;
        for k in 0..s.edges[v].len() {
            let w = s.edges[v][k];
            match s.index[w] {
                None => {
                    visit(s, w);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(i) if s.on_stack[w] => s.low[v] = s.low[v].min(i),
                Some(_) => {}
            }
        }
        if Some(s.low[v]) == s.index[v] {
            let mut group = Vec::new();
            loop {
                let w = s.stack.pop().expect("on the stack");
                s.on_stack[w] = false;
                group.push(w);
                if w == v {
                    break;
                }
            }
            group.sort();
            s.out.push(group);
        }
    }
    let n = edges.len();
    let mut s = State {
        edges,
        index: vec![None; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next: 0,
        out: Vec::new(),
    };
    for v in 0..n {
        if s.index[v].is_none() {
            visit(&mut s, v);
        }
    }
    s.out
}

/// Does effect set `t`, or a set in it, have a placeholder in it?
pub(super) fn has_placeholder(t: &Type) -> bool {
    matches!(t, Type::Inferred(..)) || t.parts().any(has_placeholder)
}

/// Why function `.0` has an effect, if it's inferred: the first site in
/// its body that has it, and the function that site calls, if it's one
/// whose effects are inferred too, which has it for a reason of its own.
#[derive(Debug, Clone)]
pub(super) struct Witness {
    pub span: Span,
    pub what: String,
    pub callee: Option<String>,
}

impl Checker {
    /// The placeholder for the effects of `id`, inferred: at its own
    /// parameters, in its own signature.
    pub(super) fn placeholder(&self, symbol: &str, params: &crate::types::Params) -> Type {
        let args = params.as_slice().iter().map(|p| Type::Param(p.clone()));
        Type::effects([Type::Inferred(symbol.to_string(), args.collect())])
    }

    /// `t` with each placeholder whose function's effects are known by
    /// now replaced with them, at its arguments, as `solution` (for those
    /// being worked out) or what's been inferred so far says.
    fn expand(&self, t: &Type, solution: &HashMap<String, Vec<Type>>) -> Type {
        if !has_placeholder(t) {
            return t.clone();
        }
        let t = t.map(|p| self.expand(p, solution));
        let Type::Effects(items) = &t else {
            return t;
        };
        let mut out = Vec::new();
        for e in items {
            let Type::Inferred(symbol, args) = e else {
                out.push(e.clone());
                continue;
            };
            let known = match solution.get(symbol) {
                Some(items) => Type::effects(items.iter().cloned()),
                None => match self.inferred.get(symbol) {
                    Some(t) => t.clone(),
                    None => {
                        out.push(e.clone());
                        continue;
                    }
                },
            };
            let def = self
                .fns
                .iter()
                .find(|f| f.symbol == *symbol)
                .expect("a function");
            let s = Subst::new(&def.scheme.params, args);
            out.push(self.solved(&s.apply(&known)));
        }
        Type::effects(out)
    }

    /// The effects of `t` with what's known of each placeholder in it put
    /// in: what a function's effects inferred so far say.
    pub(super) fn with_inferred(&self, t: &Type) -> Type {
        self.expand(t, &HashMap::new())
    }

    /// Works out the effects of the functions in `group`, whose bodies are
    /// all checked by now, and those of the groups before it, if any had
    /// to wait: the least that holds for every site in each body. Then
    /// checks what waited for them, and gives each its effects in its
    /// type, so calls of it from here on have them as if written.
    ///
    /// A group whose sites have a placeholder of a function not in it nor
    /// done waits for it: that's only for a call the names didn't show
    /// (see `body_groups`), and every body's done at the end.
    pub(super) fn solve_group(&mut self, group: Vec<FnId>, last: bool) -> Result<(), Error> {
        self.waiting.extend(group);
        let members: HashSet<String> = (self.waiting.iter())
            .map(|f| self.fns[f.0].symbol.clone())
            .collect();
        let open = |t: &Type| -> bool {
            fn go(t: &Type, members: &HashSet<String>, done: &HashMap<String, Type>) -> bool {
                match t {
                    Type::Inferred(s, _) if !members.contains(s) && !done.contains_key(s) => true,
                    t => t.parts().any(|p| go(p, members, done)),
                }
            }
            go(t, &members, &self.inferred)
        };
        let sites: Vec<(FnId, &Site)> = (self.waiting.iter())
            .flat_map(|f| self.infer_sites[&f.0].iter().map(move |s| (*f, s)))
            .collect();
        if !last && sites.iter().any(|(_, s)| open(&s.effects)) {
            return Ok(());
        }
        let mut solution: HashMap<String, Vec<Type>> =
            members.iter().map(|s| (s.clone(), Vec::new())).collect();
        let mut witness: HashMap<(String, Type), Witness> = HashMap::new();
        loop {
            let mut changed = false;
            for (f, site) in &sites {
                let symbol = &self.fns[f.0].symbol;
                let effects = self.expand(&site.effects, &solution);
                for e in effects.effect_items() {
                    let have = &solution[symbol];
                    if have.contains(e) {
                        continue;
                    }
                    // The function it calls, if that has `e` for a reason of its
                    // own: an inferred one, done or in this group.
                    let callee = (site.callee.clone())
                        .filter(|c| self.inferred.contains_key(c) || members.contains(c))
                        .or_else(|| callee_of(&site.effects, e, |t| self.expand(t, &solution)));
                    witness.insert(
                        (symbol.clone(), e.clone()),
                        Witness {
                            span: site.span,
                            what: site.what.clone(),
                            callee,
                        },
                    );
                    let have = solution.get_mut(symbol).expect("a member");
                    have.push(e.clone());
                    changed = true;
                    // Only a call of itself at bigger types, round the
                    // group, which isn't allowed (docs/04-generics.md, checked
                    // once all bodies are), makes them grow without end: end
                    // that here, as nothing else would.
                    if have.len() > MOST_EFFECTS {
                        let name = &self.fns[f.0].name;
                        return Err(Error::ty(
                            name.span,
                            format!(
                                "the effects of `{}` grow without end: it's called at bigger \
                                 and bigger types by the functions it calls; say them with `with`",
                                name.name
                            ),
                        ));
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // What's never inferred: whichever of the group has it first.
        for f in &self.waiting {
            let def = &self.fns[f.0];
            if !solution[&def.symbol].contains(&Type::Unsafe) {
                continue;
            }
            let w = &witness[&(def.symbol.clone(), Type::Unsafe)];
            let name = &def.name.name;
            let message = format!(
                "{} is unsafe, but it isn't in an `unsafe` block, and `{name}` doesn't say `with \
                 unsafe`",
                w.what
            );
            let note = format!(
                "`{name}`'s effects are inferred, as it doesn't say `with` and isn't exported, \
                 but never `unsafe`: that its callers keep obligations is a promise only its \
                 signature can make"
            );
            return Err(Error::ty(w.span, message).with_note(def.name.span, note));
        }
        for (symbol, items) in &solution {
            let def = self
                .fns
                .iter()
                .position(|f| f.symbol == *symbol)
                .expect("a function");
            let t = Type::effects(
                items
                    .iter()
                    .filter(|e| !matches!(e, Type::Inferred(..)))
                    .cloned(),
            );
            self.inferred.insert(symbol.clone(), t.clone());
            if let Type::Fn(sig) = &mut self.fns[def].scheme.ty {
                *sig.effects = t;
            }
        }
        self.witnesses.extend(witness);
        let waiting = std::mem::take(&mut self.waiting);
        for f in &waiting {
            self.infer_sites.remove(&f.0);
        }
        // What waited for these, and is known now.
        let deferred = std::mem::take(&mut self.deferred);
        for mut d in deferred {
            d.sub = self.with_inferred(&d.sub);
            d.sup = self.with_inferred(&d.sup);
            if !last && (has_placeholder(&d.sub) || has_placeholder(&d.sup)) {
                self.deferred.push(d);
                continue;
            }
            self.check_deferred(&d)?;
        }
        Ok(())
    }

    /// Replaces every placeholder left in what's kept of the package being
    /// checked with what was inferred, once it all is: in the notes, the
    /// instances, and the types of closures and of what opaque types
    /// stand for, which monomorphising and the tools read.
    pub(super) fn zonk_inferred(&mut self) {
        let z = |cx: &Checker, t: &Type| cx.with_inferred(t);
        let mut notes = std::mem::take(&mut self.info.notes);
        for n in &mut notes {
            n.ty = n.ty.as_ref().map(|t| z(self, t));
            n.args = n.args.iter().map(|t| z(self, t)).collect();
        }
        self.info.notes = notes;
        let mut instances = std::mem::take(&mut self.info.instances);
        for i in instances.values_mut() {
            match i {
                Instance::Fn(_, args) | Instance::Adt(_, args) => {
                    *args = args.iter().map(|t| z(self, t)).collect();
                }
            }
        }
        self.info.instances = instances;
        let mut closures = std::mem::take(&mut self.traits.closures);
        for c in closures.values_mut() {
            if let Type::Fn(sig) = z(self, &Type::Fn(c.sig.clone())) {
                c.sig = sig;
            }
            for (_, t) in &mut c.fields {
                *t = z(self, t);
            }
        }
        self.traits.closures = closures;
        let mut opaques = std::mem::take(&mut self.traits.opaques);
        for o in opaques.values_mut() {
            o.hidden = o.hidden.as_ref().map(|t| z(self, t));
        }
        self.traits.opaques = opaques;
        let mut opaques = std::mem::take(&mut self.adts.proj.opaques);
        for o in opaques.values_mut() {
            o.hidden = o.hidden.as_ref().map(|t| z(self, t));
        }
        self.adts.proj.opaques = opaques;
        self.info.inferred = self.inferred.clone();
    }

    /// Why inferred function `symbol` has effect `e`, call by call, as
    /// notes: each where it has it, and what's there.
    pub(super) fn why_inferred(&self, symbol: &str, e: &Type) -> Vec<(Span, String)> {
        let mut notes = Vec::new();
        let mut at = symbol.to_string();
        let mut seen = HashSet::new();
        while seen.insert(at.clone())
            && let Some(w) = self.witnesses.get(&(at.clone(), e.clone()))
        {
            let name = super::scope::source_name(&at);
            let does = super::effects::does(&Type::effects([e.clone()]));
            let does = does.trim_start_matches("may ");
            notes.push((w.span, format!("`{name}` may {does} here: {} may", w.what)));
            match &w.callee {
                Some(next) => at = next.clone(),
                None => break,
            }
        }
        notes
    }
}

/// Of the placeholders in `site`, a site's effects, the function of the
/// first whose effects, as `expand` has them, have `e`, if any does.
fn callee_of(site: &Type, e: &Type, expand: impl Fn(&Type) -> Type) -> Option<String> {
    site.effect_items().iter().find_map(|i| match i {
        Type::Inferred(symbol, _)
            if expand(&Type::effects([i.clone()]))
                .effect_items()
                .contains(e) =>
        {
            Some(symbol.clone())
        }
        _ => None,
    })
}

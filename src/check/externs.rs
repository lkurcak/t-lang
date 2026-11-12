//! `extern fn` (docs/40-extern.md): functions with the C calling
//! convention, under their own names. An *import* has no body: it's a C
//! function, whose calls are `unsafe`, as what it does is C's. An *export*
//! has one: a T function C can call.
//!
//! What crosses is only C's scalars (E3): the integers, `bool`, and as a
//! result `()` and `!`. Every bit pattern of one of those is a value of its
//! T type, so whatever C passes an export is one, and an import's result
//! needs no promise beyond its C signature. Nothing else does, as T makes
//! no promise of how anything else is laid out.

use super::*;

/// The C functions the runtime needs from the platform that a program may
/// write in T, as exports (lemma P of docs/40-extern.md): their names.
/// The rest of the platform, the memory, is C's: what `t_platform_alloc`
/// gives back is a promise the runtime trusts, which no safe T could make.
pub(crate) const PLATFORM_HOOKS: [&str; 2] = ["t_platform_write", "t_platform_halt"];

/// The parameters and result of the platform's function `name`, if it's
/// one of `PLATFORM_HOOKS`, as the runtime calls it.
fn platform_hook(name: &str) -> Option<(Vec<Type>, Type)> {
    match name {
        "t_platform_write" => Some((vec![Type::Int(crate::types::IntTy::U8)], Type::Unit)),
        "t_platform_halt" => Some((Vec::new(), Type::Never)),
        _ => None,
    }
}

/// What an `extern fn` is, as checking its signature found it.
#[derive(Debug, Clone)]
pub(super) struct ExternFn {
    pub fn_id: FnId,
    /// Its name, which is its C name, where it's declared.
    pub name: Ident,
    /// Is it an import, which has no body?
    pub import: bool,
    /// Is it one of `PLATFORM_HOOKS`?
    pub hook: bool,
}

impl Checker {
    /// Checks the signature of `f`, an `extern fn` (`ext`) whose type is
    /// `scheme`, and gives back its type: an import's calls are `unsafe`
    /// too, whatever it says.
    pub(super) fn extern_signature(
        &mut self,
        prog: &Program,
        f: &FnDecl,
        ext: Extern,
        i: usize,
        mut scheme: Scheme,
    ) -> Result<Scheme, Error> {
        // A C name is the program's, all of it: two libraries couldn't each
        // be sure of theirs (E4). Whether a package is a program is its
        // own, so this is the same whether it's checked alone or in a
        // program (lemma P2 of docs/26-packages.md).
        let root = self.modules.package_root(f.module);
        if (prog.packages.iter()).any(|p| p.root == root && p.library) {
            return Err(Error::ty(
                ext.span,
                "only a program can have an `extern fn`, not a library: a C name is one for the \
                 whole program, which no library in it could be sure is its own",
            ));
        }
        let g = &f.generics;
        if let Some(p) = g.lifetimes.first().or(g.types.first()) {
            return Err(Error::ty(
                p.span,
                "an `extern fn` has no generic parameters: C calls it, or it's C, as it is",
            ));
        }
        let name = &f.name.name;
        let hook = platform_hook(name);
        if name == "main" || (name.starts_with("t_") && hook.is_none()) {
            return Err(Error::ty(
                f.name.span,
                format!(
                    "`{name}` is a name of the runtime's, in C: an `extern fn` can't be called \
                     `main`, or start with `t_`"
                ),
            ));
        }
        let Type::Fn(sig) = &mut scheme.ty else {
            unreachable!("a function's type")
        };
        for (p, t) in f.params.iter().zip(&sig.params) {
            if let Some(at) = self.refined_at(&p.ty).filter(|_| !ext.import) {
                return Err(not_proved(at, ext));
            }
            if !matches!(t, Type::Int(_) | Type::Bool) {
                return Err(Error::ty(
                    p.ty.span,
                    format!(
                        "an `extern fn` takes only integers and `bool`s, which are what C's are, \
                         not `{t}`"
                    ),
                ));
            }
        }
        if let Some(at) = self.refined_at(&f.ret).filter(|_| ext.import) {
            return Err(not_proved(at, ext));
        }
        if !matches!(
            &*sig.ret,
            Type::Int(_) | Type::Bool | Type::Unit | Type::Never
        ) {
            return Err(Error::ty(
                f.ret.span,
                format!(
                    "an `extern fn` gives back an integer, a `bool`, `()` or `!`, which are what \
                     C's can, not `{}`",
                    sig.ret
                ),
            ));
        }
        if let Some(e) = f.ensures.first() {
            return Err(Error::ty(
                e.span,
                "an `extern fn` promises nothing with `ensures`: it has no `&mut` to promise of",
            ));
        }
        if ext.import {
            if hook.is_some() {
                return Err(Error::ty(
                    f.name.span,
                    format!(
                        "`{name}` is the platform's, which the runtime calls: a program can write \
                         it, with a body, but not declare it"
                    ),
                ));
            }
            // What C does is its own: that it keeps to what the import says
            // it does is its caller's to know (E1).
            let effects = sig.effects.effect_items().iter().cloned();
            *sig.effects = Type::effects(effects.chain([Type::Unsafe]));
        } else {
            // Nothing in T calls it for C to keep its obligations (E2).
            let unsafe_effect = (f.effects.iter().flat_map(|e| &e.items))
                .find(|e| matches!(&e.kind, EffectKind::Named(n) if n.name == "unsafe"));
            if let Some(e) = unsafe_effect {
                return Err(Error::ty(
                    e.span,
                    "an exported `extern fn` can't say `with unsafe`: C calls it, which keeps no \
                     obligations",
                ));
            }
            // Checked the same for either target (docs/71-t-metal-findings.md):
            // a run or a build for the hosted one refuses it, as the C
            // library's is the platform there (`crate::hosted`).
            if let Some((params, ret)) = &hook
                && (sig.params != *params || *sig.ret != *ret)
            {
                let params: Vec<String> = params.iter().map(|t| t.to_string()).collect();
                let ret = match ret {
                    Type::Unit => String::new(),
                    t => format!(" -> {t}"),
                };
                return Err(Error::ty(
                    f.name.span,
                    format!(
                        "the platform's `{name}` is a `fn({}){ret}`, as the runtime calls it",
                        params.join(", ")
                    ),
                ));
            }
        }
        if let Some(other) = self.externs.iter().find(|e| e.name.name == *name) {
            return Err(Error::ty(
                f.name.span,
                format!(
                    "there's another `extern fn {name}`: a C name is one for the whole program"
                ),
            )
            .with_note(other.name.span, "it's here"));
        }
        self.externs.push(ExternFn {
            fn_id: FnId(i),
            name: f.name.clone(),
            import: ext.import,
            hook: hook.is_some(),
        });
        Ok(scheme)
    }

    /// Checks, once effects are inferred, that what C may call when it's
    /// in the middle of something doesn't need what it's in the middle
    /// of. An export may be called from an interrupt, while the code it
    /// interrupted is allocating, and the allocator can't be started again
    /// before it ends, so it can't allocate (lemma E2); that it frees
    /// nothing follows, as it owns nothing on the heap. The platform's
    /// functions do nothing that needs the platform (lemma P): a panic in
    /// `t_platform_write` would write, and allocating would need memory,
    /// which may be why it's writing. Using a static is fine: a device's
    /// state can be in one.
    pub(super) fn externs_known(&mut self) -> Result<(), Error> {
        for e in self.externs.iter().filter(|e| !e.import) {
            let Type::Fn(sig) = &self.fns[e.fn_id.0].scheme.ty else {
                continue;
            };
            let effects = self.with_inferred(&sig.effects);
            let allowed = |t: &Type| match e.hook {
                true => *t == Type::Global,
                false => matches!(t, Type::Global | Type::Panic),
            };
            let Some(bad) = (effects.effect_items().iter()).find(|t| !allowed(t)) else {
                continue;
            };
            let name = &e.name.name;
            let why = match e.hook {
                true => format!(
                    "the platform's `{name}` may only use statics: the runtime calls it while it \
                     panics, or runs out of memory, so it can't have `{bad}`"
                ),
                false => format!(
                    "C may call `{name}` from an interrupt, in the middle of an allocation, which \
                     can't be started again before it ends: an export can't have `{bad}`"
                ),
            };
            return Err(Error::ty(e.name.span, why));
        }
        Ok(())
    }
}

impl Checker {
    /// Where `t` is refined, if it is: `{ x: u32 | x > 0 }`, or an alias
    /// of one (`Checker::written_out`).
    fn refined_at(&mut self, t: &TypeExpr) -> Option<Span> {
        let written = self.written_out(t);
        matches!(written.kind, TypeExprKind::Refined(..)).then_some(t.span)
    }
}

/// That a refinement at `at`, on `extern fn` `ext`, can't be: C proves
/// nothing.
fn not_proved(at: Span, ext: Extern) -> Error {
    let message = match ext.import {
        true => {
            "an imported `extern fn` gives back no refinement: what a C function gives back \
                 is only what its C type says"
        }
        false => {
            "an exported `extern fn` takes no refinement: C calls it, and proves nothing of \
                  what it passes"
        }
    };
    Error::ty(at, message)
}

//! Source-level explanations of conservative proof failures. These only
//! observe the walk's state; they never add facts or change a proof.

use super::*;

#[derive(Debug, Clone)]
pub(in crate::check) struct ProofFailure {
    reason: Failure,
    condition: String,
    facts: Vec<String>,
    invalidated: Vec<(Span, String)>,
}

impl ProofFailure {
    pub(super) fn attach(&self, mut error: Error) -> Error {
        let span = error.span;
        error = error.with_note(span, format!("required condition: {}", self.condition));
        error = error.with_note(span, self.reason.explanation());
        if !self.facts.is_empty() {
            error = error.with_note(span, format!("known here: {}", self.facts.join("; ")));
        }
        for (at, note) in &self.invalidated {
            error = error.with_note(*at, note);
        }
        error
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Invalidation {
    place: Place,
    span: Span,
    cause: &'static str,
    /// A possible change collected before walking the loop, rather than
    /// an operation already evaluated on the current path.
    loop_span: Option<Span>,
}

impl Invalidation {
    pub(super) fn new(place: Place, span: Span, cause: &'static str) -> Self {
        Self {
            place,
            span,
            cause,
            loop_span: None,
        }
    }

    pub(super) fn place(&self) -> &Place {
        &self.place
    }
}

impl Checker {
    pub(in crate::check) fn explain_proof(&self, error: Error) -> Error {
        match self.proof_failures.get(&error.span) {
            Some(failure) => failure.attach(error),
            None => error,
        }
    }

    /// A closure's inferred `panic` may be rejected at the closure's
    /// bound rather than at an operator. Explain its first failed proof.
    pub(in crate::check) fn explain_closure_proof(&self, error: Error) -> Error {
        let failure = self
            .proof_failures
            .iter()
            .filter(|(span, _)| error.span.start <= span.start && span.end <= error.span.end)
            .min_by_key(|(span, _)| (span.start, span.end));
        match failure {
            Some((_, failure)) => failure.attach(error),
            None => error,
        }
    }
}

impl Walk<'_> {
    pub(super) fn invalidate(&mut self, place: &Place, span: Span, cause: &'static str) {
        self.record_invalidation(Invalidation::new(place.clone(), span, cause));
    }

    pub(super) fn invalidate_loop(&mut self, change: &Invalidation, loop_span: Span) {
        let mut event = change.clone();
        event.loop_span = Some(loop_span);
        self.record_invalidation(event);
    }

    fn record_invalidation(&mut self, event: Invalidation) {
        if self
            .cur
            .keys()
            .chain(self.objs.keys())
            .any(|p| p.within(&event.place) || event.place.within(p))
        {
            self.invalidated.push(event);
        }
    }

    pub(super) fn operator_proof(
        &mut self,
        site: NodeId,
        goal: Option<Formula>,
        missing: Failure,
        fallback: &str,
    ) {
        let outcome = match goal.as_ref() {
            None if self.reachable => Err(missing),
            goal => self.proves(goal),
        };
        match outcome {
            Ok(()) => {
                self.proved.insert(site);
            }
            Err(reason) => {
                let failure = self.proof_failure(reason, goal.as_ref(), None, fallback);
                self.failures.insert(site, failure);
            }
        }
    }

    fn place_name(&self, p: &Place) -> Option<String> {
        let bind = self.cx.typed.binds.get(&p.root)?;
        let mut name = bind.name.clone();
        let mut ty = self.cx.solved(&bind.ty);
        for proj in &p.path {
            match proj {
                Proj::Deref => {
                    name = format!("(*{name})");
                    ty = match ty {
                        Type::Ref(_, _, inner) | Type::Box(inner) => self.cx.solved(&inner),
                        _ => return None,
                    };
                }
                Proj::Field(i) => {
                    let fields = struct_fields(&self.cx.adts, &ty)?;
                    let (field, ft) = fields.get(*i)?;
                    name = format!("{name}.{field}");
                    ty = self.cx.solved(ft);
                }
            }
        }
        Some(name)
    }

    /// Names only the values that still belong to these places. Old SSA
    /// values deliberately have no current source name after a write.
    fn names(&self) -> (HashMap<Var, String>, HashMap<Var, Place>) {
        let (mut names, mut places) = (HashMap::new(), HashMap::new());
        let mut current: Vec<_> = self.cur.iter().collect();
        current.sort_by_key(|(p, _)| *p);
        for (p, v) in current {
            if let Some(name) = self.place_name(p) {
                names.insert(*v, name);
                places.insert(*v, p.clone());
            }
        }
        let mut objects: Vec<_> = self.objs.iter().collect();
        objects.sort_by_key(|(p, _)| *p);
        for (p, obj) in objects {
            let Some(name) = self.place_name(p) else {
                continue;
            };
            let mut measures: Vec<_> = self
                .measured
                .iter()
                .filter(|((o, _), _)| o == obj)
                .collect();
            measures.sort_by_key(|((_, symbol), _)| symbol.as_str());
            for ((_, symbol), value) in measures {
                if let Val::Int(l) = value
                    && let Some(v) = l.as_var()
                {
                    let method = &self.cx.measures[symbol].name;
                    names
                        .entry(v)
                        .or_insert_with(|| format!("{name}.{method}()"));
                    places.entry(v).or_insert_with(|| p.clone());
                }
            }
        }
        if let Some(v) = self.size {
            names.insert(v, "core::num::greatest::<isize>()".into());
        }
        for (p, (lo, hi)) in &self.bounds {
            for (bound, function) in [(lo, "least"), (hi, "greatest")] {
                if let Some(v) = bound.as_var() {
                    names.insert(v, format!("core::num::{function}::<{p}>()"));
                }
            }
        }
        for (name, val) in &self.consts {
            if let Val::Int(l) = val
                && let Some(v) = l.as_var()
            {
                names.insert(v, name.clone());
            }
        }
        (names, places)
    }

    pub(super) fn proof_failure(
        &self,
        reason: Failure,
        goal: Option<&Formula>,
        value: Option<&Val>,
        fallback: &str,
    ) -> ProofFailure {
        let (mut names, places) = self.names();
        if let Some(Val::Int(l)) = value
            && let Some(v) = l.as_var()
        {
            names.entry(v).or_insert_with(|| "the value".into());
        }
        // Shadowed bindings can both still occur in the facts. A source
        // name may describe only one value, preferably the goal's value.
        let mut goal_vars = Vec::new();
        if let Some(g) = goal {
            g.vars(&mut goal_vars);
        }
        let (mut counts, mut in_goal) = (HashMap::new(), HashMap::new());
        for (v, name) in &names {
            *counts.entry(name.clone()).or_insert(0) += 1;
            if goal_vars.contains(v) {
                *in_goal.entry(name.clone()).or_insert(0) += 1;
            }
        }
        names.retain(|v, name| {
            counts[name] == 1 || (goal_vars.contains(v) && in_goal.get(name) == Some(&1))
        });
        let name = |v| names.get(&v).cloned();
        let condition = goal
            .and_then(|g| g.text(&name))
            .unwrap_or_else(|| fallback.into());
        let all: Vec<_> = self.facts.iter().chain(&self.laws).cloned().collect();
        let relevant = goal.map(|g| logic::relevant(&all, g)).unwrap_or_default();
        let mut vars = Vec::new();
        if let Some(g) = goal {
            g.vars(&mut vars);
        }
        for f in &relevant {
            f.vars(&mut vars);
        }
        vars.sort();
        vars.dedup();
        let mut facts = Vec::new();
        let mut include = |text: String| {
            if text != "true" && text.len() <= 180 && facts.len() < 6 && !facts.contains(&text) {
                facts.push(text);
            }
        };
        for fact in relevant {
            if let Some(text) = fact.text(&name) {
                include(text);
            }
        }
        // Type ranges are implicit solver facts, and often exactly what
        // makes a missing upper or lower bound clear to the programmer.
        for &v in &vars {
            if let Kind::Int(lo, hi) = &self.kinds[v as usize] {
                for bound in [Formula::le(lo, &Lin::var(v)), Formula::le(&Lin::var(v), hi)]
                    .into_iter()
                    .flatten()
                {
                    if let Some(text) = bound.text(&name) {
                        include(text);
                    }
                }
            }
        }
        let mut invalidated = Vec::new();
        let mut seen = HashSet::new();
        let mut loops = HashSet::new();
        let mut selected = 0;
        for event in self.invalidated.iter().rev() {
            if !seen.insert(&event.place) {
                continue;
            }
            if vars
                .iter()
                .filter_map(|v| places.get(v))
                .any(|p| p.within(&event.place) || event.place.within(p))
                && let Some(name) = self.place_name(&event.place)
            {
                if let Some(span) = event.loop_span {
                    invalidated.push((
                        event.span,
                        format!("{} in this loop may change `{name}`", event.cause),
                    ));
                    if loops.insert(span) {
                        invalidated.push((
                            span,
                            "facts from before this loop are not assumed for every iteration"
                                .into(),
                        ));
                    }
                } else {
                    invalidated.push((
                        event.span,
                        format!(
                            "{} invalidated facts about the previous value of `{name}`",
                            event.cause
                        ),
                    ));
                }
                selected += 1;
                if selected == 2 {
                    break;
                }
            }
        }
        ProofFailure {
            reason,
            condition,
            facts,
            invalidated,
        }
    }
}

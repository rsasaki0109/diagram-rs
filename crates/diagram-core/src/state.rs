//! State diagram IR with distinct initial/final pseudostates and transitions.
use crate::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateKind {
    Regular,
    Initial,
    Final,
    Choice,
    Fork,
    Join,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    pub label: String,
    pub description: String,
    pub kind: StateKind,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub source: usize,
    pub target: usize,
    pub label: String,
    pub span: Span,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StateDiagram {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub direction: Direction,
    pub config: Config,
}
impl StateDiagram {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        validate_config(&self.config)?;
        let fail = |s| {
            Diagnostic::new(
                s,
                Span {
                    line: 1,
                    column: 1,
                    ..Span::default()
                },
            )
        };
        if self.states.is_empty()
            || self.states.len() > MAX_NODES
            || self.transitions.len() > MAX_EDGES
        {
            return Err(fail("state diagram exceeds limits or has no states"));
        }
        let mut ids = BTreeSet::new();
        for s in &self.states {
            if s.id.is_empty() || !ids.insert(&s.id) {
                return Err(Diagnostic::new("empty or duplicate state id", s.span));
            }
            for t in [&s.id, &s.label, &s.description] {
                validate_text(t, s.span)?;
            }
            validate_text(&format!("{}\n{}", s.label, s.description), s.span)?;
        }
        for t in &self.transitions {
            if t.source >= self.states.len() || t.target >= self.states.len() {
                return Err(Diagnostic::new("invalid transition endpoint", t.span));
            }
            if self.states[t.source].kind == StateKind::Final
                || self.states[t.target].kind == StateKind::Initial
            {
                return Err(Diagnostic::new(
                    "transition cannot leave a final state or enter an initial state",
                    t.span,
                ));
            }
            validate_text(&t.label, t.span)?;
        }
        Ok(())
    }
}

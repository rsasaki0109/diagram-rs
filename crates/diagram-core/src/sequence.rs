//! Sequence diagram IR. Event order is time order; participant indices are stable.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantKind {
    Participant,
    Actor,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub kind: ParticipantKind,
    pub span: Span,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    SolidArrow,
    DottedArrow,
    SolidOpen,
    DottedOpen,
    SolidCross,
    DottedCross,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotePosition {
    LeftOf,
    RightOf,
    Over,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FragmentKind {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SequenceEvent {
    Message {
        source: usize,
        target: usize,
        label: String,
        kind: MessageKind,
        span: Span,
    },
    Activate {
        participant: usize,
        span: Span,
    },
    Deactivate {
        participant: usize,
        span: Span,
    },
    Note {
        participants: Vec<usize>,
        position: NotePosition,
        label: String,
        span: Span,
    },
    FragmentStart {
        kind: FragmentKind,
        label: String,
        span: Span,
    },
    Branch {
        label: String,
        span: Span,
    },
    FragmentEnd {
        span: Span,
    },
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub participants: Vec<Participant>,
    pub events: Vec<SequenceEvent>,
    pub autonumber: bool,
    pub config: Config,
}
impl Sequence {
    /// Validate directly constructed/deserialized IR, including temporal scopes.
    pub fn validate(&self) -> Result<(), Diagnostic> {
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
        if self.participants.is_empty()
            || self.participants.len() > MAX_NODES
            || self.events.len() > MAX_EDGES
        {
            return Err(fail(
                "sequence requires 1..=512 participants and at most 2048 events",
            ));
        }
        validate_config(&self.config)?;
        let mut ids = BTreeSet::new();
        for p in &self.participants {
            if p.id.is_empty() || !ids.insert(&p.id) {
                return Err(Diagnostic::new("empty or duplicate participant id", p.span));
            }
            validate_text(&p.id, p.span)?;
            validate_text(&p.label, p.span)?;
        }
        let mut active = vec![0_usize; self.participants.len()];
        let mut fragments = Vec::new();
        let endpoint = |i: usize, span: Span| {
            if i >= self.participants.len() {
                Err(Diagnostic::new("invalid participant index", span))
            } else {
                Ok(())
            }
        };
        for e in &self.events {
            match e {
                SequenceEvent::Message {
                    source,
                    target,
                    label,
                    span,
                    ..
                } => {
                    endpoint(*source, *span)?;
                    endpoint(*target, *span)?;
                    validate_text(label, *span)?;
                }
                SequenceEvent::Activate { participant, span } => {
                    endpoint(*participant, *span)?;
                    active[*participant] += 1;
                    if active[*participant] > 32 {
                        return Err(Diagnostic::new("activation depth exceeds 32", *span));
                    }
                }
                SequenceEvent::Deactivate { participant, span } => {
                    endpoint(*participant, *span)?;
                    if active[*participant] == 0 {
                        return Err(Diagnostic::new(
                            "cannot deactivate an inactive participant",
                            *span,
                        ));
                    }
                    active[*participant] -= 1;
                }
                SequenceEvent::Note {
                    participants,
                    position,
                    label,
                    span,
                } => {
                    if participants.is_empty()
                        || participants.len() > 2
                        || (*position != NotePosition::Over && participants.len() != 1)
                    {
                        return Err(Diagnostic::new(
                            "note requires one participant, or two for over",
                            *span,
                        ));
                    }
                    for &p in participants {
                        endpoint(p, *span)?;
                    }
                    validate_text(label, *span)?;
                }
                SequenceEvent::FragmentStart { kind, label, span } => {
                    if fragments.len() == 32 {
                        return Err(Diagnostic::new("fragment depth exceeds 32", *span));
                    }
                    fragments.push(*kind);
                    validate_text(label, *span)?;
                }
                SequenceEvent::Branch { label, span } => {
                    if !matches!(
                        fragments.last(),
                        Some(FragmentKind::Alt | FragmentKind::Par | FragmentKind::Critical)
                    ) {
                        return Err(Diagnostic::new(
                            "branch requires alt, par or critical",
                            *span,
                        ));
                    }
                    validate_text(label, *span)?;
                }
                SequenceEvent::FragmentEnd { span } => {
                    if fragments.pop().is_none() {
                        return Err(Diagnostic::new("unexpected sequence end", *span));
                    }
                }
            }
        }
        if !fragments.is_empty() {
            return Err(fail("unclosed sequence fragment; expected end"));
        }
        if active.iter().any(|&n| n != 0) {
            return Err(fail("unclosed activation; expected deactivate"));
        }
        Ok(())
    }
}

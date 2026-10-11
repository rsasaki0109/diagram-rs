//! Entity relationship IR with identifying links, cardinalities and keyed attributes.
use crate::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    ExactlyOne,
    ZeroOrOne,
    OneOrMore,
    ZeroOrMore,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeKey {
    PK,
    FK,
    UK,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub data_type: String,
    pub name: String,
    pub keys: Vec<AttributeKey>,
    pub comment: String,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub source: usize,
    pub target: usize,
    pub source_cardinality: Cardinality,
    pub target_cardinality: Cardinality,
    pub identifying: bool,
    pub label: String,
    pub span: Span,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ErDiagram {
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
    pub direction: Direction,
    pub config: Config,
}
impl ErDiagram {
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
        if self.entities.is_empty()
            || self.entities.len() > MAX_NODES
            || self.relationships.len() > MAX_EDGES
        {
            return Err(fail("ER diagram exceeds limits or has no entities"));
        }
        let mut ids = BTreeSet::new();
        for e in &self.entities {
            if e.id.is_empty() || !ids.insert(&e.id) {
                return Err(Diagnostic::new("empty or duplicate entity id", e.span));
            }
            validate_text(&e.id, e.span)?;
            if e.attributes.len() > 256 {
                return Err(Diagnostic::new("entity exceeds 256 attributes", e.span));
            }
            let mut names = BTreeSet::new();
            for a in &e.attributes {
                if a.data_type.is_empty() || a.name.is_empty() || !names.insert(&a.name) {
                    return Err(Diagnostic::new(
                        "empty type/name or duplicate attribute",
                        a.span,
                    ));
                }
                for t in [&a.name, &a.data_type, &a.comment] {
                    validate_text(t, a.span)?;
                }
                let mut keys = BTreeSet::new();
                for k in &a.keys {
                    if !keys.insert(format!("{k:?}")) {
                        return Err(Diagnostic::new("duplicate attribute key", a.span));
                    }
                }
            }
        }
        for r in &self.relationships {
            if r.source >= self.entities.len() || r.target >= self.entities.len() {
                return Err(Diagnostic::new("invalid ER relationship endpoint", r.span));
            }
            validate_text(&r.label, r.span)?;
        }
        Ok(())
    }
}

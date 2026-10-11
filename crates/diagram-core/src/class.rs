//! Class diagram IR retains compartments and relationship semantics.
use crate::*;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Class {
    pub id: String,
    pub label: String,
    pub annotation: Option<String>,
    pub attributes: Vec<String>,
    pub methods: Vec<String>,
    pub span: Span,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassRelationKind {
    Inheritance,
    Realization,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Link,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClassRelation {
    pub source: usize,
    pub target: usize,
    pub kind: ClassRelationKind,
    pub reversed: bool,
    pub dashed: bool,
    pub source_cardinality: String,
    pub target_cardinality: String,
    pub label: String,
    pub span: Span,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<ClassRelation>,
    pub direction: Direction,
    pub config: Config,
}
impl ClassDiagram {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        validate_config(&self.config)?;
        if self.classes.is_empty()
            || self.classes.len() > MAX_NODES
            || self.relations.len() > MAX_EDGES
        {
            return Err(Diagnostic::new(
                "class diagram exceeds limits or has no classes",
                Span {
                    line: 1,
                    column: 1,
                    ..Span::default()
                },
            ));
        }
        let mut ids = BTreeSet::new();
        for c in &self.classes {
            if c.id.is_empty() || !ids.insert(&c.id) {
                return Err(Diagnostic::new("empty or duplicate class id", c.span));
            }
            for t in [&c.id, &c.label]
                .into_iter()
                .chain(c.annotation.iter())
                .chain(c.attributes.iter())
                .chain(c.methods.iter())
            {
                validate_text(t, c.span)?;
            }
            let title = match &c.annotation {
                Some(a) => format!("«{a}»\n{}", c.label),
                None => c.label.clone(),
            };
            validate_text(
                &format!(
                    "{title}\n{}\n{}",
                    c.attributes.join("\n"),
                    c.methods.join("\n")
                ),
                c.span,
            )?;
            if c.attributes.len() + c.methods.len() > 256 {
                return Err(Diagnostic::new("class exceeds 256 members", c.span));
            }
        }
        for r in &self.relations {
            if r.source >= self.classes.len() || r.target >= self.classes.len() {
                return Err(Diagnostic::new(
                    "invalid class relationship endpoint",
                    r.span,
                ));
            }
            for t in [&r.label, &r.source_cardinality, &r.target_cardinality] {
                validate_text(t, r.span)?;
            }
            if r.source_cardinality.len() > 32 || r.target_cardinality.len() > 32 {
                return Err(Diagnostic::new(
                    "cardinality label exceeds 32 bytes",
                    r.span,
                ));
            }
        }
        Ok(())
    }
}

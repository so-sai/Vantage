use super::label::CognitiveLabel;

#[derive(Debug, Clone)]
pub struct Seal {
    pub sealed_by: String,
    pub sealed_at_unix: i64,
}

#[derive(Debug, Clone)]
pub struct CognitiveClaim {
    pub id: String,
    pub label: CognitiveLabel,
    pub sealed: Option<Seal>,
}

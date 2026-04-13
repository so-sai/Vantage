use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityDescriptor {
    pub name: &'static str,
    pub inputs: &'static [&'static str],
    pub outputs: &'static [&'static str],
    pub invariants: &'static [&'static str],
    pub help: &'static str,
}

pub trait VantageIntrospect: Send + Sync {
    fn capability_name() -> &'static str;
    fn input_schema() -> &'static str;
    fn output_schema() -> &'static str;
    fn invariants() -> &'static [&'static str];
    fn help_text() -> &'static str;
}

pub struct VantageCapabilityRegistry {
    pub entries: &'static [CapabilityDescriptor],
}

impl VantageCapabilityRegistry {
    pub fn list(&self) -> &[CapabilityDescriptor] {
        self.entries
    }

    pub fn get(&self, name: &str) -> Option<&CapabilityDescriptor> {
        self.entries.iter().find(|e| e.name == name)
    }
}

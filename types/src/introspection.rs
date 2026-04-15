use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SystemEnvelope {
    pub version: &'static str,
    pub safe_node_limit: u32,
    pub nonlinear_boundary: u32,
    pub deterministic: bool,
    pub zero_copy: bool,
    pub o_n_log_n_guarantee: bool,
}

impl SystemEnvelope {
    pub fn current() -> Self {
        Self {
            version: "v1.2.4",
            safe_node_limit: 500,
            nonlinear_boundary: 2500,
            deterministic: true,
            zero_copy: true,
            o_n_log_n_guarantee: true,
        }
    }
}

impl Default for SystemEnvelope {
    fn default() -> Self {
        Self::current()
    }
}

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

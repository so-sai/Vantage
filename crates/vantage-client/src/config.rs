use vantage_pek::ProofPolicy;

#[derive(Debug, Clone)]
pub enum ClientMode {
    Embedded,
    Remote { base_url: String, api_key: Option<String> },
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub identity: String,
    pub policy: ProofPolicy,
    pub mode: ClientMode,
}

impl ClientConfig {
    pub fn embedded(identity: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            policy: ProofPolicy::Enforced,
            mode: ClientMode::Embedded,
        }
    }

    pub fn remote(identity: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            policy: ProofPolicy::Enforced,
            mode: ClientMode::Remote {
                base_url: base_url.into(),
                api_key: None,
            },
        }
    }

    pub fn with_policy(mut self, policy: ProofPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        if let ClientMode::Remote { api_key, .. } = &mut self.mode {
            *api_key = Some(key.into());
        }
        self
    }
}

use super::claim::{ClaimType, CognitiveClaim};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Warn,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub decision: Decision,
    pub claim_id: String,
    pub reason: String,
}

pub trait Invariant {
    fn check(&self, claim: &CognitiveClaim) -> Option<Verdict>;
}

pub struct InvariantRule {
    pub name: String,
    pub claim_type: ClaimType,
    pub decision: Decision,
    pub reason: String,
}

impl Invariant for InvariantRule {
    fn check(&self, claim: &CognitiveClaim) -> Option<Verdict> {
        if claim.claim_type == self.claim_type {
            Some(Verdict {
                decision: self.decision,
                claim_id: claim.id.clone(),
                reason: self.reason.clone(),
            })
        } else {
            None
        }
    }
}

pub struct InvariantEngine {
    rules: Vec<Box<dyn Invariant>>,
}

impl InvariantEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: Box<dyn Invariant>) {
        self.rules.push(rule);
    }

    pub fn evaluate(&self, claim: &CognitiveClaim) -> Verdict {
        for rule in &self.rules {
            if let Some(verdict) = rule.check(claim) {
                return verdict;
            }
        }
        Verdict {
            decision: Decision::Allow,
            claim_id: claim.id.clone(),
            reason: "no_rule_matched".to_string(),
        }
    }

    pub fn evaluate_all(&self, claims: &[CognitiveClaim]) -> Vec<Verdict> {
        claims.iter().map(|c| self.evaluate(c)).collect()
    }

    pub fn has_reject(&self, claims: &[CognitiveClaim]) -> bool {
        self.evaluate_all(claims)
            .iter()
            .any(|v| v.decision == Decision::Reject)
    }
}

impl Default for InvariantEngine {
    fn default() -> Self {
        Self::new()
    }
}

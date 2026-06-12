use std::time::SystemTime;
use vantage_core::{
    AgentId, CommitReceipt, KnowledgeMutation, MutationId, MutationOp, ResourceId,
};
use crate::client::VantageClient;
use crate::error::VantageError;

pub struct MutationBuilder<'a> {
    client: &'a mut VantageClient,
    resource_id: String,
    mutation_id: Option<String>,
    payload: Option<String>,
    is_delete: bool,
}

impl<'a> MutationBuilder<'a> {
    pub fn new(client: &'a mut VantageClient) -> Self {
        Self {
            client,
            resource_id: String::new(),
            mutation_id: None,
            payload: None,
            is_delete: false,
        }
    }

    pub fn resource(mut self, id: impl Into<String>) -> Self {
        self.resource_id = id.into();
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.mutation_id = Some(id.into());
        self
    }

    pub fn insert(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self.is_delete = false;
        self
    }

    pub fn delete(mut self) -> Self {
        self.is_delete = true;
        self.payload = None;
        self
    }

    pub fn execute(self) -> Result<CommitReceipt, VantageError> {
        let resource_id = ResourceId(self.resource_id);
        let mutation_id = self.mutation_id.unwrap_or_else(|| {
            format!("mut_{}", uuid_v4_simple())
        });
        let actor = AgentId(self.client.config().identity.clone());

        let mutation = KnowledgeMutation {
            mutation_id: MutationId(mutation_id),
            actor,
            op: if self.is_delete {
                MutationOp::Delete { resource_id }
            } else {
                MutationOp::Insert {
                    resource_id,
                    payload: self.payload.unwrap_or_default(),
                }
            },
            timestamp: SystemTime::now(),
        };

        self.client.execute(mutation)
    }
}

fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

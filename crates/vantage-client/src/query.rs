use vantage_core::ResourceId;
use crate::client::VantageClient;
use crate::error::{HistoryEntry, VantageError};

pub struct QueryBuilder<'a> {
    client: &'a VantageClient,
    resource_id: String,
}

impl<'a> QueryBuilder<'a> {
    pub fn new(client: &'a VantageClient) -> Self {
        Self {
            client,
            resource_id: String::new(),
        }
    }

    pub fn resource(mut self, id: impl Into<String>) -> Self {
        self.resource_id = id.into();
        self
    }

    pub fn exists(self) -> Result<bool, VantageError> {
        let id = ResourceId(self.resource_id);
        self.client.exists(&id)
    }

    pub fn current(self) -> Result<Option<String>, VantageError> {
        let id = ResourceId(self.resource_id);
        self.client.read_unit(&id)
    }

    pub fn history(self) -> Result<Vec<HistoryEntry>, VantageError> {
        let id = ResourceId(self.resource_id);
        self.client.history(&id)
    }
}

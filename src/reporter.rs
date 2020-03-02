mod slack;
mod stdout;

pub use slack::SlackReporter;
pub use stdout::StdoutReporter;

use async_trait::async_trait;
use crate::response_summary::ConnectorType;

#[async_trait]
pub trait Reporter {
    async fn from_sqlite(&self, path: &str, connector: ConnectorType) -> crate::Result<()>;
}

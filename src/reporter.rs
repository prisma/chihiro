mod slack;
mod stdout;

pub use slack::SlackReporter;
pub use stdout::StdoutReporter;

use crate::response_summary::ConnectorType;
use async_trait::async_trait;

#[async_trait]
pub trait Reporter {
    async fn report(&self, path: &str, connector: ConnectorType) -> crate::Result<()>;
}

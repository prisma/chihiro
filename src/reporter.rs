mod slack;
mod stdout;

pub use slack::SlackReporter;
pub use stdout::StdoutReporter;

use async_trait::async_trait;

#[async_trait]
pub trait Reporter {
    async fn from_sqlite(&self, path: &str, connector: &str) -> crate::Result<()>;
}

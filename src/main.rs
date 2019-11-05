mod requester;

use requester::Requester;
use structopt::StructOpt;
use metrics_runtime::{Receiver, observers::PrometheusBuilder};
use metrics_core::{Builder, Observe, Drain};

#[derive(Debug, StructOpt, Clone)]
/// Prisma Load Tester
pub struct Opt {
    /// Number of requests per second.
    #[structopt(long)]
    rate: u64,
    /// Number of seconds to run.
    #[structopt(long)]
    duration: Option<u64>,
    /// Request timeout in seconds.
    #[structopt(long)]
    timeout: Option<u64>,
    #[structopt(long)]
    prisma_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opt::from_args();

    let receiver = Receiver::builder().build()?;
    let cont = receiver.controller();
    receiver.install();

    let mut requester = Requester::from(opts);
    requester.run().await?;

    let mut observer = PrometheusBuilder::new().build();
    cont.observe(&mut observer);

    println!("{}", observer.drain());

    Ok(())
}

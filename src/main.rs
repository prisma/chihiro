mod query_loader;
mod requester;
mod console_observer;

use query_loader::QueryConfig;
use requester::Requester;
use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle};
use console::{pad_str, Alignment};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, StructOpt, Clone)]
/// Prisma Load Tester
pub struct Opt {
    /// Request timeout in seconds.
    #[structopt(long)]
    timeout: Option<u64>,
    /// The Prisma URL. Default: http://localhost:4466/
    #[structopt(long)]
    prisma_url: Option<String>,
    /// The query configuration file (toml) to execute.
    #[structopt(long)]
    query_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opt::from_args();

    let query_config = QueryConfig::new(opts.query_file)?;
    let mut requester = Requester::new(opts.prisma_url)?;

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.cyan} [{elapsed_precise}] {wide_msg}");

    let tests = query_config.test_count();
    let pad_len = query_config.longest_name_length();

    for (i, (query, rate)) in query_config.queries().enumerate() {
        let pb = ProgressBar::new(query_config.duration().as_secs());

        pb.set_prefix(&format!(
            "[{}/{}] {}",
            i + 1,
            tests,
            pad_str(query.name(), pad_len, Alignment::Left, None)
        ));

        pb.set_style(spinner_style.clone());

        requester.run(query.query(), rate, query_config.duration(), pb).await?;
        requester.clear_metrics()?;
    }

    Ok(())
}

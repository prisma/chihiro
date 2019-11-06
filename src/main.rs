mod query_loader;
mod requester;
mod console_observer;

use query_loader::QueryConfig;
use requester::Requester;
use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle};
use console::style;
use futures::stream::TryStreamExt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, StructOpt, Clone)]
/// Prisma Load Tester
pub struct Opt {
    /// The Prisma URL. Default: http://localhost:4466/
    #[structopt(long)]
    prisma_url: Option<String>,
    /// The query configuration file (toml) to execute.
    #[structopt(long)]
    query_file: String,
    #[structopt(long)]
    validate: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opt::from_args();

    let query_config = QueryConfig::new(opts.query_file)?;
    let mut requester = Requester::new(opts.prisma_url)?;

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.cyan} [{elapsed_precise:.bold.dim}] {wide_msg}");

    if opts.validate {
        println!("Validating queries...");
        let pb = ProgressBar::new(query_config.query_count() as u64);

        for query in query_config.queries() {
            let res = requester.request(query.query()).await?;
            let body = res.into_body().try_concat().await?;
            let body = String::from_utf8(body.to_vec())?;
            let json: serde_json::Value = serde_json::from_str(&body)?;

            if json["errors"] != serde_json::Value::Null {
                panic!("Query {} returned an error: {}", query.name(), json);
            }

            pb.inc(1);
        }

        pb.finish_with_message("All queries validated");
    }

    let tests = query_config.test_count();
    for (i, (query, rate)) in query_config.runs().enumerate() {
        println!(
            "[{}] {}",
            style(&format!("{}/{}", i+1, tests)).bold().dim(),
            query.name(),
        );

        let pb = ProgressBar::new(query_config.duration().as_secs());

        pb.set_style(spinner_style.clone());

        requester.run(query.query(), rate, query_config.duration(), pb).await?;
        requester.clear_metrics()?;
    }

    Ok(())
}

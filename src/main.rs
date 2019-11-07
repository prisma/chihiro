mod config;
mod requester;
mod console_observer;
mod bar;

use config::QueryConfig;
use bar::OptionalBar;
use requester::Requester;
use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle};
use console::style;

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
    #[structopt(long)]
    show_progress: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opt::from_args();

    let query_config = QueryConfig::new(&opts.query_file)?;
    let mut requester = Requester::new(opts.prisma_url)?;

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.cyan} [{elapsed_precise:.bold.dim}] {wide_msg}");

    if opts.validate {
        let pb = if opts.show_progress {
            OptionalBar::from(ProgressBar::new(query_config.query_count() as u64))
        } else {
            OptionalBar::empty()
        };

        println!("Validating queries...");
        requester.validate(&query_config, pb).await.unwrap();
    }

    let tests = query_config.test_count();

    for (i, query) in query_config.queries().enumerate() {
        for rate in query.rps() {
            println!(
                "[{}] {}",
                style(&format!("{}/{}", i+1, tests)).bold().dim(),
                query.name(),
            );

            let pb = if opts.show_progress {
                OptionalBar::from(ProgressBar::new(query_config.duration().as_secs()))
            } else {
                OptionalBar::empty()
            };

            pb.set_style(spinner_style.clone());

            requester.run(&query, *rate, query_config.duration(), pb).await?;
            requester.clear_metrics()?;
        }
    }

    Ok(())
}

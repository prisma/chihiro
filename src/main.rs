mod bar;
mod config;
mod console_observer;
mod requester;

use bar::OptionalBar;
use config::QueryConfig;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use requester::Requester;
use structopt::StructOpt;
use tokio::runtime::Runtime;

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

fn main() -> Result<()> {
    let opts = Opt::from_args();

    let query_config = QueryConfig::new(&opts.query_file)?;

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.cyan} [{elapsed_precise:.bold.dim}] {wide_msg}");

    if opts.validate {
        let requester = Requester::new(opts.prisma_url.clone())?;
        let rt = Runtime::new()?;

        rt.block_on(async {
            let show_progress = opts.show_progress;

            let pb = if show_progress {
                OptionalBar::from(ProgressBar::new(query_config.query_count() as u64))
            } else {
                OptionalBar::empty()
            };

            println!("Validating queries...");
            requester.validate(&query_config, pb).await.unwrap();
        });

        rt.shutdown_now();
    }

    let tests = query_config.test_count();

    for (i, (query, rate)) in query_config.runs().enumerate() {
        let requester = Requester::new(opts.prisma_url.clone())?;
        let rt = Runtime::new()?;

        let pb = if opts.show_progress {
            OptionalBar::from(ProgressBar::new(query_config.duration().as_secs()))
        } else {
            OptionalBar::empty()
        };

        println!(
            "[{}] {}",
            style(&format!("{}/{}", i + 1, tests)).bold().dim(),
            query.name(),
        );

        pb.set_style(spinner_style.clone());

        rt.block_on(async {
            requester
                .run(&query, rate, query_config.duration(), &pb)
                .await;
        });

        pb.finish_with_message(&requester.metrics());

        rt.shutdown_now();
    }

    Ok(())
}

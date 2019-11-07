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
    #[structopt(long)]
    show_progress: bool,
}

pub struct OptionalBar {
    inner: Option<ProgressBar>,
}

impl From<ProgressBar> for OptionalBar {
    fn from(pb: ProgressBar) -> Self {
        Self { inner: Some(pb) }
    }
}

impl OptionalBar {
    pub fn empty() -> Self {
        Self { inner: None }
    }

    pub fn set_style(&self, style: ProgressStyle) {
        if let Some(ref inner) = self.inner {
            inner.set_style(style);
        }
    }

    pub fn inc(&self, num: u64) {
        if let Some(ref inner) = self.inner {
            inner.inc(num);
        }
    }

    pub fn set_message(&self, msg: &str) {
        if let Some(ref inner) = self.inner {
            inner.set_message(msg);
        }
    }

    pub fn finish_with_message(&self, msg: &str) {
        if let Some(ref inner) = self.inner {
            inner.finish_with_message(msg);
        }
    }
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

        let pb = if opts.show_progress {
            OptionalBar::from(ProgressBar::new(query_config.query_count() as u64))
        } else {
            OptionalBar::empty()
        };

        for query in query_config.queries() {
            let res = requester.request(&query).await?;
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

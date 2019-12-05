use crate::{bar, config::QueryConfig, metrics_sender::MetricsSender, requester::Requester};
use bar::OptionalBar;
use chrono::Duration;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::{env, io};
use tokio::runtime::Runtime;

pub struct Bench {
    opts: crate::BenchOpt,
    query_config: QueryConfig,
    metrics_sender: MetricsSender,
    spinner: ProgressStyle,
}

impl Bench {
    pub fn new(opts: crate::BenchOpt) -> crate::Result<Self> {
        let query_config = QueryConfig::new(&opts.query_file)?;

        let elastic_user = env::var("ELASTIC_USER")
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ELASTIC_USER not set"))?;

        let elastic_password = env::var("ELASTIC_PW")
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ELASTIC_PW not set"))?;

        let metrics_sender = MetricsSender::new(
            query_config.elastic_endpoint(),
            &opts.metrics_database,
            &elastic_user,
            &elastic_password,
        );

        let spinner = ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{prefix:.bold.cyan} [{elapsed_precise:.bold.dim}] {wide_msg}");

        Ok(Self {
            opts,
            query_config,
            metrics_sender,
            spinner,
        })
    }

    pub fn run(&self) -> crate::Result<()> {
        self.print_info()?;

        if self.opts.validate {
            self.validate()?;
        }

        let tests = self.query_config.test_count();
        let total_tests = self.query_config.test_count();

        let total_time = Duration::seconds(
            (self.query_config.duration().as_secs() * (total_tests as u64)) as i64,
        );

        let total_hours = total_time.num_hours();
        let total_minutes = total_time.num_minutes() - total_hours * 60;

        println!(
            "Running {} tests, {} seconds for each. Ready in about {} and {}...",
            style(&format!("{}", total_tests)).bold(),
            style(&format!("{}", self.query_config.duration().as_secs())).bold(),
            style(&format!("{} hour(s)", total_hours)).bold(),
            style(&format!("{} minute(s)", total_minutes)).bold(),
        );

        for (i, (query, rps)) in self.query_config.runs().enumerate() {
            let mut rt = Runtime::new()?;

            let requester = Requester::new(self.opts.prisma_url.clone())?;

            let pb = if self.opts.show_progress {
                OptionalBar::from(ProgressBar::new(self.query_config.duration().as_secs()))
            } else {
                OptionalBar::empty()
            };

            println!(
                "[{}] {} ({} rps)",
                style(&format!("{}/{}", i + 1, tests)).bold().dim(),
                query.name(),
                rps,
            );

            pb.set_style(self.spinner.clone());

            rt.block_on(async {
                requester
                    .run(&query, rps, self.query_config.duration(), &pb)
                    .await;

                let metrics = requester.json_metrics(query.name(), rps).await?;
                self.metrics_sender.send(&metrics).await
            })?;

            pb.finish_with_message(&requester.console_metrics());
        }

        Ok(())
    }

    fn print_info(&self) -> crate::Result<()> {
        let requester = Requester::new(self.opts.prisma_url.clone())?;
        let mut rt = Runtime::new()?;

        rt.block_on(async {
            let info = requester.server_info().await?;

            println!(
                "Server info :: commit: {}, version: {}, primary_connector: {}",
                style(&format!("{}", info.commit)).bold(),
                style(&format!("{}", info.version)).bold(),
                style(&format!("{}", info.primary_connector)).bold(),
            );

            Ok::<(), Box<dyn std::error::Error>>(())
        })?;

        Ok(())
    }

    fn validate(&self) -> crate::Result<()> {
        let requester = Requester::new(self.opts.prisma_url.clone())?;
        let mut rt = Runtime::new()?;

        rt.block_on(async {
            let show_progress = self.opts.show_progress;

            let pb = if show_progress {
                OptionalBar::from(ProgressBar::new(self.query_config.query_count() as u64))
            } else {
                OptionalBar::empty()
            };

            println!("Validating queries...");
            requester.validate(&self.query_config, pb).await.unwrap();
        });

        Ok(())
    }
}

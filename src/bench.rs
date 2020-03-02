use crate::{
    bar, config::QueryConfig, metrics_sender::MetricsSender, metrics_storage::MetricsStorage,
    requester::Requester,
};
use bar::OptionalBar;
use chrono::Duration;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::{env, io};

pub struct Bench {
    opts: crate::BenchOpt,
    query_config: QueryConfig,
    metrics_sender: MetricsSender,
    metrics_storage: MetricsStorage,
    spinner: ProgressStyle,
    requester: Requester,
}

impl Bench {
    pub async fn new(opts: crate::BenchOpt) -> crate::Result<Self> {
        let requester = Requester::new(opts.endpoint_type, opts.endpoint_url.clone())?;
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

        let metrics_storage = MetricsStorage::new(&opts.sqlite_path).await?;

        let spinner = ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{prefix:.bold.cyan} [{elapsed_precise:.bold.dim}] {wide_msg}");

        Ok(Self {
            opts,
            query_config,
            metrics_sender,
            metrics_storage,
            spinner,
            requester,
        })
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        self.print_info().await?;

        if self.opts.validate {
            self.validate().await?;
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

            self.requester
                .run(&query, rps, self.query_config.duration(), &pb)
                .await;

            let metrics = self.requester.json_metrics(query.name(), rps).await?;
            self.metrics_sender.send(&metrics).await?;
            self.metrics_storage.store(&metrics).await?;

            println!("{}", self.requester.console_metrics());
        }

        Ok(())
    }

    async fn print_info(&self) -> crate::Result<()> {
        let info = self.requester.server_info().await?;

        println!(
            "Server info :: commit: {}, version: {}, primary_connector: {}",
            style(&format!("{}", info.commit)).bold(),
            style(&format!("{}", info.version)).bold(),
            style(&format!("{}", info.primary_connector)).bold(),
        );

        Ok(())
    }

    async fn validate(&self) -> crate::Result<()> {
        let show_progress = self.opts.show_progress;
        let pb = if show_progress {
            OptionalBar::from(ProgressBar::new(self.query_config.query_count() as u64))
        } else {
            OptionalBar::empty()
        };

        println!("Validating queries...");
        self.requester.validate(&self.query_config, pb).await?;

        Ok(())
    }
}

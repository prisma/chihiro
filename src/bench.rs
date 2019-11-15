use bar::OptionalBar;
use crate::{
    bar,
    config::QueryConfig,
    requester::Requester,
    metrics_sender::MetricsSender
};
use console::style;
use chrono::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::runtime::Runtime;
use std::env;

pub fn run(opts: crate::BenchOpt) -> crate::Result<()> {
    let query_config = QueryConfig::new(&opts.query_file)?;

    let elastic_user = env::var("ELASTIC_USER").expect("ELASTIC_USER not set");
    let elastic_password = env::var("ELASTIC_PW").expect("ELASTIC_PW not set");

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

    let metrics_sender = MetricsSender::new(
        query_config.elastic_endpoint(),
        &opts.metrics_database,
        &elastic_user,
        &elastic_password,
    );

    let total_tests = query_config.test_count();
    let total_time = Duration::seconds((query_config.duration().as_secs() * (total_tests as u64)) as i64);
    let total_hours = total_time.num_hours();
    let total_minutes = total_time.num_minutes() - total_hours * 60;

    println!(
        "Running {} tests, {} seconds for each. Ready in about {} and {}...",
        style(&format!("{}", total_tests)).bold(),
        style(&format!("{}", query_config.duration().as_secs())).bold(),
        style(&format!("{} hour(s)", total_hours)).bold(),
        style(&format!("{} minute(s)", total_minutes)).bold(),
    );

    for (i, (query, rps)) in query_config.runs().enumerate() {
        let rt = Runtime::new()?;

        let requester = Requester::new(opts.prisma_url.clone())?;

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
                .run(&query, rps, query_config.duration(), &pb)
                .await;

            let metrics = requester.json_metrics(query.name(), rps).await?;
            metrics_sender.send(&metrics).await
        })?;

        pb.finish_with_message(&requester.console_metrics());
        rt.shutdown_now();
    }

    Ok(())
}

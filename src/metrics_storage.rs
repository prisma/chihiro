use crate::json_observer::ResponseTime;
use quaint::{prelude::*, single::Quaint};

pub struct MetricsStorage {
    db: Quaint,
}

impl MetricsStorage {
    pub async fn new(path: &str) -> crate::Result<Self> {
        Ok(Self {
            db: Quaint::new(path).await?,
        })
    }

    pub async fn store(&self, metrics: &ResponseTime) -> crate::Result<()> {
        let previous_version = Select::from_table("version")
            .so_that("commit_id".equals(metrics.commit()))
            .and_where("connector".equals(metrics.connector()));

        let version = match self.db.select(previous_version).await?.first() {
            Some(result) => result["id"].as_i64().unwrap(),
            None => {
                let insert = Insert::single_into("version")
                    .value("commit_id", metrics.commit())
                    .value("version", metrics.version())
                    .value("connector", metrics.connector());

                let result = self.db.insert(insert.into()).await?;

                result.last_insert_id().unwrap() as i64
            }
        };

        let insert = Insert::single_into("response_time")
            .value("version", version)
            .value("time", metrics.time())
            .value("failures", metrics.failures() as i64)
            .value("p50", metrics.p50() as i64)
            .value("p95", metrics.p95() as i64)
            .value("p99", metrics.p99() as i64)
            .value("query_name", metrics.query_name())
            .value("rps", metrics.rps() as i64)
            .value("successes", metrics.successes() as i64);

        self.db.insert(insert.into()).await?;

        Ok(())
    }
}

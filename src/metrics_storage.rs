use crate::{json_observer::ResponseTime, requester::ServerInfo};
use quaint::{prelude::*, single::Quaint};

pub struct MetricsStorage {
    db: Quaint,
}

impl MetricsStorage {
    pub async fn new(uri: &str) -> crate::Result<Self> {
        Ok(Self {
            db: Quaint::new(uri).await?,
        })
    }

    pub async fn contains(&self, info: &ServerInfo) -> crate::Result<bool> {
        let query = Select::from_table("version")
            .so_that("commit_id".equals(info.commit.as_str()))
            .and_where("connector".equals(info.primary_connector.as_str()));

        let result = self
            .db
            .select(query)
            .await?
            .first()
            .map(|_| true)
            .unwrap_or(false);

        Ok(result)
    }

    pub async fn store(&self, metrics: &ResponseTime) -> crate::Result<()> {
        let previous_version = Select::from_table("version")
            .so_that("commit_id".equals(metrics.commit()))
            .and_where("connector".equals(metrics.connector()));

        let version = match self.db.select(previous_version).await?.first() {
            Some(result) => result["id"].as_i64().unwrap(),
            None => {
                let insert_single = Insert::single_into("version")
                    .value("commit_id", metrics.commit())
                    .value("version", metrics.version())
                    .value("connector", metrics.connector());

                let result = self
                    .db
                    .insert(Insert::from(insert_single).returning(vec!["id"]))
                    .await?;

                result
                    .first()
                    .map(|row| row["id"].as_i64().unwrap())
                    .unwrap()
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

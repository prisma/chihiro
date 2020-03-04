use crate::error::Error;
use quaint::{ast::avg, prelude::*, single::Quaint};
use serde::Deserialize;
use std::{collections::BTreeMap, str::FromStr, string::ToString};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectorType {
    Postgres,
    Mysql,
}

impl ToString for ConnectorType {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

impl ConnectorType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
        }
    }
}

impl FromStr for ConnectorType {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "postgres" | "postgresql" => Ok(Self::Postgres),
            "mysql" => Ok(Self::Mysql),
            typ => Err(Error::InvalidDatabaseType(typ.into())),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ResponseAverage {
    query_name: String,
    commit_id: String,
    p50: f64,
    p95: f64,
    p99: f64,
}

#[derive(Debug, Default)]
pub struct ResponseSummary {
    previous_averages: BTreeMap<String, ResponseAverage>,
    next_averages: BTreeMap<String, ResponseAverage>,
}

impl ResponseSummary {
    pub async fn aggregate(url: &str, connector: ConnectorType) -> crate::Result<Self> {
        let db = Quaint::new(url).await?;

        let selected_versions = Select::from_table("version")
            .column("id")
            .so_that("connector".equals(connector.as_str()))
            .order_by("id".descend())
            .limit(2);

        let select = Select::from_table("response_time")
            .column(Column::from(("response_time", "query_name")).alias("query_name"))
            .column(Column::from(("version", "commit_id")).alias("commit_id"))
            .value(Function::from(avg(("response_time", "p50"))).alias("p50"))
            .value(Function::from(avg(("response_time", "p95"))).alias("p95"))
            .value(Function::from(avg(("response_time", "p99"))).alias("p99"))
            .inner_join(
                "version".on(("version", "id").equals(Column::from(("response_time", "version")))),
            )
            .so_that(Column::from(("version", "id")).in_selection(selected_versions))
            .group_by(Column::from(("response_time", "query_name")))
            .group_by(Column::from(("version", "commit_id")))
            .group_by(Column::from(("version", "id")))
            .order_by(Column::from(("version", "id")).descend())
            .order_by(Column::from(("response_time", "query_name")).ascend());

        let times: Vec<ResponseAverage> = quaint::serde::from_rows(db.select(select).await?)?;
        let mut summary = Self::default();

        for time in times.into_iter() {
            summary.insert(time);
        }

        if summary.next_averages.is_empty() {
            Err(Error::NotEnoughMeasurements(connector.to_string()))
        } else {
            Ok(summary)
        }
    }

    pub fn differences(&self) -> Vec<(&str, Option<(f64, f64, f64)>)> {
        self.next_averages
            .iter()
            .map(|(key, next)| {
                let query_name = next.query_name.as_str();
                match self.previous_averages.get(key) {
                    Some(previous) => (
                        query_name,
                        Some((
                            (1.0 - next.p50 / previous.p50) * 100.0,
                            (1.0 - next.p95 / previous.p95) * 100.0,
                            (1.0 - next.p99 / previous.p99) * 100.0,
                        )),
                    ),
                    None => (query_name, None),
                }
            })
            .collect()
    }

    pub fn longest_query(&self) -> usize {
        self.next_averages
            .iter()
            .max_by(|x, y| x.0.len().cmp(&y.0.len()))
            .unwrap()
            .0
            .len()
    }

    fn insert(&mut self, value: ResponseAverage) {
        let key = value.query_name.clone();

        if self.previous_averages.contains_key(&key) {
            self.next_averages.insert(key, value);
        } else {
            self.previous_averages.insert(key, value);
        }
    }

    pub fn commits(&self) -> (&str, &str) {
        let next = self
            .next_averages
            .iter()
            .next()
            .unwrap()
            .1
            .commit_id
            .as_str();

        let previous = self
            .previous_averages
            .iter()
            .next()
            .unwrap()
            .1
            .commit_id
            .as_str();

        (previous, next)
    }
}

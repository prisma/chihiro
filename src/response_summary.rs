use quaint::{ast::avg, prelude::*, serde::from_rows, single::Quaint};
use serde::Deserialize;
use std::{collections::BTreeMap, io};

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
    pub async fn find_from_sqlite(path: &str) -> crate::Result<Self> {
        let db = Quaint::new(path).await?;

        let inner_select = Select::from_table("version")
            .column("id")
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
            .so_that(Column::from(("version", "id")).in_selection(inner_select))
            .group_by(Column::from(("response_time", "query_name")))
            .group_by(Column::from(("version", "commit_id")))
            .order_by(Column::from(("version", "id")).descend())
            .order_by(Column::from(("response_time", "query_name")).ascend());

        let times: Vec<ResponseAverage> = from_rows(db.select(select).await?)?;
        let mut summary = Self::default();

        for time in times.into_iter() {
            summary.insert(time);
        }

        if summary.next_averages.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Not enough measurements",
            ))?
        } else {
            Ok(summary)
        }
    }

    pub fn differences(&self) -> Vec<(&str, f64, f64, f64)> {
        self.next_averages
            .iter()
            .map(|(key, next)| {
                let previous = self.previous_averages.get(key).unwrap();

                (
                    next.query_name.as_str(),
                    (1.0 - next.p50 / previous.p50) * 100.0,
                    (1.0 - next.p95 / previous.p95) * 100.0,
                    (1.0 - next.p99 / previous.p99) * 100.0,
                )
            })
            .collect()
    }

    pub fn longest_query(&self) -> usize {
        self.next_averages
            .iter()
            .fold(0, |acc, (key, _)| match acc {
                x if key.len() > x => key.len(),
                _ => acc,
            })
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

use super::Reporter;
use crate::response_summary::{ResponseSummary, ConnectorType};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

pub struct SlackReporter {
    webhook_url: String,
    client: Client,
}

impl SlackReporter {
    pub fn new(webhook_url: &str) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            client: Client::new(),
        }
    }

    fn format_number(num: f64) -> String {
        if num < 0.0 {
            format!("{:.2}%", num)
        } else {
            format!("*_{:.2}%_*", num)
        }
    }

    fn format_title(num: f64, text: &str) -> String {
        if num < 0.0 {
            format!(":heavy_check_mark:*{}*", text)
        } else {
            format!(":x:*{}*", text)
        }
    }
}

#[async_trait]
impl Reporter for SlackReporter {
    async fn from_sqlite(&self, path: &str, connector: ConnectorType) -> crate::Result<()> {
        let summary = ResponseSummary::find_from_sqlite(path, connector).await?;
        let (previous_id, next_id) = summary.commits();

        let overview = format!(
            "Benchmark results for *{}* connector, comparing commit_id `{}` against commit_id `{}`. (<https://github.com/prisma/prisma-engines/compare/{}...{}|Changelog>)",
            "postgres",
            &previous_id[0..6],
            &next_id[0..6],
            &previous_id,
            &next_id,
        );

        let mut blocks = Vec::new();

        blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": overview
            },
        }));

        blocks.push(json!({
            "type": "divider"
        }));

        for (query, p50, p95, p99) in summary.differences() {
            let p50_title = Self::format_title(p50, "p50");
            let p95_title = Self::format_title(p95, "p95");
            let p99_title = Self::format_title(p99, "p99");

            let p50 = Self::format_number(p50);
            let p95 = Self::format_number(p95);
            let p99 = Self::format_number(p99);

            let query = format!(
                "Query: <https://github.com/prisma/chihiro/blob/master/queries/sql_load_test/prisma/{}.graphql|{}>",
                query,
                query
            );

            blocks.push(json!({
                "type": "section",
                "text": {
                    "text": query,
                    "type": "mrkdwn"
                },
                "fields": [
                    {
                        "text": p50_title,
                        "type": "mrkdwn"
                    },
                    {
                        "text": p50,
                        "type": "mrkdwn"
                    },
                    {
                        "text": p95_title,
                        "type": "mrkdwn"
                    },
                    {
                        "text": p95,
                        "type": "mrkdwn"
                    },
                    {
                        "text": p99_title,
                        "type": "mrkdwn"
                    },
                    {
                        "text": p99,
                        "type": "mrkdwn"
                    }
                ]
            }));
        }

        let payload = json!({ "blocks": Value::from(blocks) });

        self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }
}

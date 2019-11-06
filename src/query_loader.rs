use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Deserialize)]
struct TestQueries {
    path: PathBuf,
    rates: Vec<u64>,
    duration: u64,
}

#[derive(Deserialize)]
struct TestConfig {
    title: String,
    queries: TestQueries,
}

#[derive(Debug)]
pub struct QueryConfig {
    queries: Vec<Query>,
    rates: Vec<u64>,
    duration: Duration,
    title: String,
}

#[derive(Debug)]
pub struct Query {
    name: String,
    query: String,
}

impl Query {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl QueryConfig {
    pub fn new<P>(test_file: P) -> crate::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut f = File::open(test_file)?;

        let mut config_str = String::new();
        f.read_to_string(&mut config_str)?;

        let config: TestConfig = toml::from_str(&config_str)?;
        let mut queries = Vec::new();

        if let Ok(entries) = config.queries.path.read_dir() {
            for entry in entries {
                let path = entry?.path();

                if let Some("graphql") = path.extension().and_then(|s| s.to_str()) {
                    let mut f = File::open(&path)?;
                    let mut query = String::new();
                    f.read_to_string(&mut query)?;

                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                        .unwrap();

                    queries.push(Query { name, query });
                }
            }
        } else {
            let mut f = File::open(&config.queries.path)?;
            let mut query = String::new();
            f.read_to_string(&mut query)?;

            let name = config
                .queries
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap();

            queries.push(Query { name, query });
        }

        Ok(Self {
            queries,
            rates: config.queries.rates,
            duration: Duration::from_secs(config.queries.duration),
            title: config.title,
        })
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn test_count(&self) -> usize {
        self.queries.len() * self.rates.len()
    }

    pub fn queries(&self) -> impl Iterator<Item = (&Query, u64)> {
        self.queries
            .iter()
            .flat_map(move |q| self.rates.iter().map(move |r| (q, *r)))
    }
}

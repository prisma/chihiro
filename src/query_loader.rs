use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use walkdir::WalkDir;

#[derive(Deserialize)]
struct TestRun {
    path: PathBuf,
    rps: Vec<u64>,
}

#[derive(Deserialize)]
struct TestConfig {
    identifier: String,
    duration_per_test: u64,
    test_runs: Vec<TestRun>,
}

#[derive(Debug)]
pub struct QueryConfig {
    queries: Vec<Query>,
    duration: Duration,
    identifier: String,
}

#[derive(Debug)]
pub struct Query {
    name: String,
    query: String,
    rps: Vec<u64>,
}

impl Query {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rps(&self) -> &[u64] {
        self.rps.as_slice()
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

        for test_run in config.test_runs {
            if test_run.path.is_dir() {
                for entry in WalkDir::new(&test_run.path) {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some("graphql") = path.extension().and_then(|s| s.to_str()) {
                        let mut f = File::open(&path)?;
                        let mut query = String::new();
                        f.read_to_string(&mut query)?;

                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                            .unwrap();

                        queries.push(Query {
                            name,
                            query,
                            rps: test_run.rps.clone(),
                        });
                    }
                }
            } else {
                let mut f = File::open(&test_run.path)?;
                let mut query = String::new();
                f.read_to_string(&mut query)?;

                let name = test_run
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .unwrap();

                queries.push(Query { name, query, rps: test_run.rps });
            }
        }

        Ok(Self {
            queries,
            duration: Duration::from_secs(config.duration_per_test),
            identifier: config.identifier,
        })
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn query_count(&self) -> usize {
        self.queries.len()
    }

    pub fn test_count(&self) -> usize {
        self.queries.iter().fold(0, |acc, q| acc + q.rps.len())
    }

    pub fn queries(&self) -> impl Iterator<Item = &Query> {
        self.queries.iter()
    }
}

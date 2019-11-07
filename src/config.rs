mod query;

pub use query::*;

use serde::Deserialize;
use std::{
    collections::HashMap,
    convert::TryFrom,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

static VERY_SLOW_RATES: &[u64] = &[25, 50, 75, 100, 125, 150, 175, 200, 225, 250];
static SLOW_RATES: &[u64] = &[50, 100, 150, 200, 250, 300, 350, 400, 450, 500];
static MEDIUM_RATES: &[u64] = &[100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 2000];
static FAST_RATES: &[u64] = &[200, 400, 600, 800, 1000, 1200, 1400, 1600, 1800, 2000, 4000];
static VERY_FAST_RATES: &[u64] = &[200, 400, 600, 800, 1000, 1200, 1400, 1600, 1800, 2000, 4000];

#[derive(Deserialize, Debug)]
pub(super) struct TestRun {
    path: PathBuf,
    #[serde(default = "HashMap::new")]
    variables: HashMap<String, QueryVariable>,
}

#[derive(Deserialize, Debug)]
pub(super) struct TestConfig {
    identifier: String,
    elastic_endpoint: String,
    duration_per_test: u64,
    test_run: Vec<TestRun>,
}

impl TryFrom<&str> for TestConfig {
    type Error = Box<dyn std::error::Error>;

    fn try_from(path: &str) -> crate::Result<Self> {
        let mut f = File::open(path)?;

        let mut config_str = String::new();
        f.read_to_string(&mut config_str)?;

        Ok(toml::from_str(&config_str)?)
    }
}

impl TestConfig {
    fn parse_name(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap()
    }

    fn rps(name: &str) -> &'static [u64] {
        if name.contains("very-slow") {
            VERY_SLOW_RATES
        } else if name.contains("slow") {
            SLOW_RATES
        } else if name.contains("medium") {
            MEDIUM_RATES
        } else if name.contains("fast") {
            FAST_RATES
        } else if name.contains("very-fast") {
            VERY_FAST_RATES
        } else {
            panic!(
                "File name should contain the query speed: (very-slow|slow|medium|fast|very-fast)"
            )
        }
    }

    pub(super) fn take_queries(&mut self) -> crate::Result<Vec<Query>> {
        let mut queries = Vec::new();

        while let Some(test_run) = self.test_run.pop() {
            if test_run.path.is_dir() {
                for entry in WalkDir::new(&test_run.path) {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some("graphql") = path.extension().and_then(|s| s.to_str()) {
                        let mut f = File::open(&path)?;
                        let mut query = String::new();
                        f.read_to_string(&mut query)?;

                        let name = Self::parse_name(path);
                        let rps = Self::rps(&name);

                        queries.push(Query {
                            name,
                            query,
                            rps,
                            variables: test_run.variables.clone(),
                        });
                    }
                }
            } else {
                let mut f = File::open(&test_run.path)?;
                let mut query = String::new();
                f.read_to_string(&mut query)?;

                let name = Self::parse_name(&test_run.path);
                let rps = Self::rps(&name);

                queries.push(Query {
                    name,
                    query,
                    rps,
                    variables: test_run.variables,
                });
            }
        }

        Ok(queries)
    }
}

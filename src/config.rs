mod query;

pub use query::*;

use serde::Deserialize;
use std::{
    collections::HashMap,
    convert::TryFrom,
    fs::File,
    io::Read,
    path::PathBuf,
};
use walkdir::WalkDir;


#[derive(Deserialize, Debug)]
pub(super) struct TestRun {
    path: PathBuf,
    rps: Vec<u64>,
    #[serde(default = "HashMap::new")]
    variables: HashMap<String, QueryVariable>,
}

#[derive(Deserialize, Debug)]
pub(super) struct TestConfig {
    identifier: String,
    duration_per_test: u64,
    test_run: Vec<TestRun>,
}

impl TryFrom<&str> for TestConfig
{
    type Error = Box<dyn std::error::Error>;

    fn try_from(path: &str) -> crate::Result<Self> {
        let mut f = File::open(path)?;

        let mut config_str = String::new();
        f.read_to_string(&mut config_str)?;

        Ok(toml::from_str(&config_str)?)
    }
}

impl TestConfig {
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

                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                            .unwrap();

                        queries.push(Query {
                            name,
                            query,
                            rps: test_run.rps.clone(),
                            variables: test_run.variables.clone(),
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

                queries.push(Query {
                    name,
                    query,
                    rps: test_run.rps,
                    variables: test_run.variables,
                });
            }
        }

        Ok(queries)
    }
}

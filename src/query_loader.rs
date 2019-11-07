use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
    collections::HashMap,
};
use walkdir::WalkDir;
use rand::Rng;

#[derive(Deserialize, Debug)]
struct TestRun {
    path: PathBuf,
    rps: Vec<u64>,
    #[serde(default = "HashMap::new")]
    variables: HashMap<String, QueryVariable>,
}

#[derive(Deserialize, Debug)]
struct TestConfig {
    identifier: String,
    duration_per_test: u64,
    test_run: Vec<TestRun>,
}

#[derive(Debug)]
pub struct QueryConfig {
    queries: Vec<Query>,
    duration: Duration,
    identifier: String,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct QueryVariable {
    minimum: u64,
    maximum: u64,
}

#[derive(Debug)]
pub struct Query {
    name: String,
    query: String,
    rps: Vec<u64>,
    variables: HashMap<String, QueryVariable>,
}

impl Query {
    pub fn query(&self) -> String {
        let mut rng = rand::thread_rng();

        self.variables.iter().fold(self.query.clone(), |acc, (name, var)| {
            let x = rng.gen_range(var.minimum, var.maximum);
            acc.replace(&format!("${}", name), &format!("{}", x))
        })
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

        for test_run in config.test_run {
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

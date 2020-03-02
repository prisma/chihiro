use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Not enough measurements in the database for connector '{}' to report differences.", _0)]
    NotEnoughMeasurements(String),
    #[error("Endpoint type '{}' is not supported", _0)]
    InvalidEndpointType(String),
    #[error("Database type '{}' is not supported", _0)]
    InvalidDatabaseType(String),
    #[error("Query {} returned an error: {}", query, error)]
    InvalidQuery { query: String, error: serde_json::Value },
    #[error("Error querying database: {}", _0)]
    Quaint(quaint::error::Error),
    #[error("IO Error: {}", _0)]
    Io(Box<dyn std::error::Error>),
    #[error("Serialization error: {}", _0)]
    Serialization(Box<dyn std::error::Error>),
    #[error("Http error: {}", _0)]
    Http(Box<dyn std::error::Error>),
    #[error("Error in SSH connection: {}", _0)]
    Ssh(ssh2::Error),
    #[error("Error in generating metrics: {}", _0)]
    MetricsError(metrics_runtime::BuilderError),
}

impl From<quaint::error::Error> for Error {
    fn from(e: quaint::error::Error) -> Self {
        Self::Quaint(e)
    }
}


impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(Box::new(e))
    }
}

impl From<walkdir::Error> for Error {
    fn from(e: walkdir::Error) -> Self {
        Self::Io(Box::new(e))
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Self::Serialization(Box::new(e))
    }
}

impl From<hyper::error::Error> for Error {
    fn from(e: hyper::error::Error) -> Self {
        Self::Http(Box::new(e))
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(Box::new(e))
    }
}

impl From<http::Error> for Error {
    fn from(e: http::Error) -> Self {
        Self::Http(Box::new(e))
    }
}

impl From<ssh2::Error> for Error {
    fn from(e: ssh2::Error) -> Self {
        Self::Ssh(e)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Self::Serialization(Box::new(e))
    }
}

impl From<uuid::Error> for Error {
    fn from(e: uuid::Error) -> Self {
        Self::Serialization(Box::new(e))
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Self::Serialization(Box::new(e))
    }
}

impl From<metrics_runtime::BuilderError> for Error {
    fn from(e: metrics_runtime::BuilderError) -> Self {
        Self::MetricsError(e)
    }
}

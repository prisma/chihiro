use indicatif::{ProgressBar, ProgressStyle};

pub struct OptionalBar {
    inner: Option<ProgressBar>,
}

impl From<ProgressBar> for OptionalBar {
    fn from(pb: ProgressBar) -> Self {
        Self { inner: Some(pb) }
    }
}

impl OptionalBar {
    pub fn empty() -> Self {
        Self { inner: None }
    }

    pub fn set_style(&self, style: ProgressStyle) {
        if let Some(ref inner) = self.inner {
            inner.set_style(style);
        }
    }

    pub fn inc(&self, num: u64) {
        if let Some(ref inner) = self.inner {
            inner.inc(num);
        }
    }

    pub fn set_message(&self, msg: &str) {
        if let Some(ref inner) = self.inner {
            inner.set_message(msg);
        }
    }

    pub fn finish_with_message(&self, msg: &str) {
        if let Some(ref inner) = self.inner {
            inner.finish_with_message(msg);
        }
    }
}

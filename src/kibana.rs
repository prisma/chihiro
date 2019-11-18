use crate::KibanaOpt;
use chrono::Utc;
use rand::Rng;
use std::{convert::TryInto, fs::File, io::Read, path::Path};
use uuid::{
    v1::{Context, Timestamp},
    Uuid,
};
use walkdir::WalkDir;

pub fn generate(opts: KibanaOpt) -> crate::Result<()> {
    let mut template = String::new();
    let mut f = File::open(&opts.template)?;
    f.read_to_string(&mut template)?;

    if opts.query_path.is_dir() {
        for entry in WalkDir::new(&opts.query_path) {
            let entry = entry?;
            let path = entry.path();

            if let Some("graphql") = path.extension().and_then(|s| s.to_str()) {
                let panel = template
                    .replace("@@query_name", &parse_name(&path))
                    .replace("@@uuid", &uuid()?);

                print!("{}", panel);
            }
        }
    } else {
        print!(
            "{}",
            template
                .replace("@@query_name", &parse_name(&opts.query_path))
                .replace("@@uuid", &uuid()?)
        );
    }

    Ok(())
}

fn uuid() -> crate::Result<String> {
    let context = Context::new(42);
    let now = Utc::now();

    let mut rng = rand::thread_rng();
    let node_id: [u8; 6] = rng.gen();

    let ts = Timestamp::from_unix(
        &context,
        now.timestamp().try_into()?,
        now.timestamp_subsec_nanos(),
    );

    Ok(Uuid::new_v1(ts, &node_id)?.to_hyphenated().to_string())
}

fn parse_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap()
}

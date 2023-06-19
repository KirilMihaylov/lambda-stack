use std::path::PathBuf;

use clap::{
    error::{Error, ErrorKind},
    Parser,
};
use zeroize::Zeroizing;

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Args {
    #[clap(short = 'u', long = "db-user")]
    pub db_user: String,
    #[clap(short = 'p', long = "db-pass")]
    pub db_pass: Zeroizing<String>,
    #[clap(short = 'k', long, default_value = "verifying.key", value_parser = file_path_parser)]
    pub verify_key: PathBuf,
    #[clap(short = 'c', long, default_value = "config.toml", value_parser = file_path_parser)]
    pub config: PathBuf,
}

fn file_path_parser(path: &str) -> Result<PathBuf, Error> {
    let path: PathBuf = PathBuf::from(path);

    if !path.is_file() {
        return Err(Error::raw(
            ErrorKind::InvalidValue,
            "Configuration file path doesn't point to a file, or no such exists!",
        ));
    }

    Ok(path)
}

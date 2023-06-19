use std::path::PathBuf;

use clap::{
    error::{Error, ErrorKind},
    Parser,
};

#[derive(Debug, Parser)]
pub struct Args {
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user: String,
    pub db_pass: String,
    #[clap(long, default_value = "auth.key", value_parser = file_path_parser)]
    pub auth_key: PathBuf,
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

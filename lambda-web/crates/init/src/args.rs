use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user: String,
    pub db_pass: String,
}

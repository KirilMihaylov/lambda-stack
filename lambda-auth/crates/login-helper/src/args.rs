use clap::Parser;
use zeroize::Zeroizing;

#[derive(Debug, Parser)]
pub struct Args {
    pub auth_uri: String,
    pub user: String,
    #[clap(value_parser = password_value)]
    pub pass: Zeroizing<String>,
}

fn password_value(password: &str) -> Result<Zeroizing<String>, clap::Error> {
    Ok(Zeroizing::new(String::from(password)))
}

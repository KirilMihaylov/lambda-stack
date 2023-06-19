use clap::Parser;

#[derive(Parser)]
pub enum Args {
    Auth {
        #[clap(short, long, default_value = "auth.key")]
        output: String,
    },
    Signing {
        #[clap(long, default_value = "signing.key")]
        signing: String,
        #[clap(long, default_value = "verifying.key")]
        verifying: String,
    },
}

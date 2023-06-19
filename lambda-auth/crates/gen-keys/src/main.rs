#![forbid(rust_2018_compatibility, deprecated_in_future)]
#![deny(rust_2021_compatibility, warnings)]

use std::fs;

use clap::Parser as _;
use ed25519_dalek::{SecretKey, SigningKey};
use opaque_ke::ServerSetup;
use rand::{rngs::OsRng, Fill as _};
use zeroize::Zeroizing;

use lambda_auth::AuthCipherSuite;

use self::args::Args;

mod args;

fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    match args {
        Args::Auth { output } => {
            fs::write(
                output,
                postcard::to_allocvec(
                    ServerSetup::<AuthCipherSuite>::new(&mut OsRng)
                        .keypair()
                        .private(),
                )?,
            )?;
        }
        Args::Signing {
            signing: output,
            verifying,
        } => {
            let signing_key: Zeroizing<SecretKey> = {
                let mut signing_key: Zeroizing<SecretKey> = Zeroizing::new(SecretKey::default());

                signing_key.try_fill(&mut OsRng)?;

                signing_key
            };

            fs::write(output, signing_key.as_slice())?;

            fs::write(
                verifying,
                SigningKey::from_bytes(&*signing_key)
                    .verifying_key()
                    .to_bytes(),
            )?;
        }
    }

    Ok(())
}

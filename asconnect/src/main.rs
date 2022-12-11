use anyhow::Result;
use asconnect::certs_api::{self, CertificateType};
use asconnect::{AppStoreConnectClient, UnifiedApiKey};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let args = Args::parse();
    args.command.run()
}

#[derive(Subcommand)]
enum Commands {
    /// Generates a PEM encoded RSA2048 signing key
    GenerateKey {
        /// Path to unified api key.
        #[clap(long)]
        api_key: PathBuf,
        /// Certificate type can be one of development, distribution or notarization.
        #[clap(long)]
        r#type: CertificateType,
        /// Path to write a new PEM encoded RSA2048 signing key
        pem: PathBuf,
    },
    /// Creates a unified api key.
    CreateApiKey {
        /// Issuer id.
        #[clap(long)]
        issuer_id: String,
        /// Key id.
        #[clap(long)]
        key_id: String,
        /// Path to private key.
        private_key: PathBuf,
        /// Path to write a unified api key.
        api_key: PathBuf,
    },
    Certificate {
        #[clap(subcommand)]
        command: CertificateCommand,
    },
}

impl Commands {
    fn run(self) -> Result<()> {
        match self {
            Self::GenerateKey {
                api_key,
                r#type,
                pem,
            } => certs_api::generate_key(&api_key, r#type, &pem)?,
            Self::CreateApiKey {
                issuer_id,
                key_id,
                private_key,
                api_key,
            } => {
                UnifiedApiKey::from_ecdsa_pem_path(issuer_id, key_id, private_key)?
                    .write_json_file(api_key)?;
            }
            Self::Certificate { command } => command.run()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum CertificateCommand {
    Create {
        /// Path to unified api key.
        #[clap(long)]
        api_key: PathBuf,
        /// Certificate type can be one of development, distribution or notarization.
        #[clap(long)]
        r#type: CertificateType,
        /// Path to certificate signing request.
        csr: PathBuf,
    },
    List {
        /// Path to unified api key.
        #[clap(long)]
        api_key: PathBuf,
    },
    Get {
        /// Path to unified api key.
        #[clap(long)]
        api_key: PathBuf,
        /// Id of certificate to revoke.
        id: String,
    },
    Revoke {
        /// Path to unified api key.
        #[clap(long)]
        api_key: PathBuf,
        /// Id of certificate to revoke.
        id: String,
    },
}

impl CertificateCommand {
    fn run(self) -> Result<()> {
        match self {
            Self::Create {
                api_key,
                csr,
                r#type,
            } => {
                let csr = std::fs::read_to_string(csr)?;
                let resp = AppStoreConnectClient::from_json_path(&api_key)?
                    .create_certificate(csr, r#type)?;
                println!(
                    "{: <10} | {: <50} | {: <20}",
                    "id", "name", "expiration date"
                );
                let expiration_date = resp
                    .data
                    .attributes
                    .expiration_date
                    .split_once('T')
                    .unwrap()
                    .0;
                println!(
                    "{: <10} | {: <50} | {: <10}",
                    resp.data.id, resp.data.attributes.name, expiration_date
                );
            }
            Self::List { api_key } => {
                let resp = AppStoreConnectClient::from_json_path(&api_key)?.list_certificates()?;
                println!(
                    "{: <10} | {: <50} | {: <20}",
                    "id", "name", "expiration date"
                );
                for cert in &resp.data {
                    let expiration_date =
                        cert.attributes.expiration_date.split_once('T').unwrap().0;
                    println!(
                        "{: <10} | {: <50} | {: <10}",
                        cert.id, cert.attributes.name, expiration_date
                    );
                }
            }
            Self::Get { api_key, id } => {
                let resp = AppStoreConnectClient::from_json_path(&api_key)?.get_certificate(&id)?;
                let cer = pem::encode(&pem::Pem {
                    tag: "CERTIFICATE".into(),
                    contents: base64::decode(&resp.data.attributes.certificate_content)?,
                });
                println!("{}", cer);
            }
            Self::Revoke { api_key, id } => {
                AppStoreConnectClient::from_json_path(&api_key)?.revoke_certificate(&id)?;
            }
        }
        Ok(())
    }
}

use anyhow::Result;
use asconnect::certs_api::{self, Certificate, CertificateType};
use asconnect::device_api::{BundleIdPlatform, Device};
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
    Device {
        #[clap(subcommand)]
        command: DeviceCommand,
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
            Self::Device { command } => command.run()?,
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
        /// Id of certificate.
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
                print_certificate_header();
                print_certificate(&resp.data);
            }
            Self::List { api_key } => {
                let resp = AppStoreConnectClient::from_json_path(&api_key)?.list_certificates()?;
                print_certificate_header();
                for cert in &resp.data {
                    print_certificate(cert);
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

fn print_certificate_header() {
    println!(
        "{: <10} | {: <50} | {: <20}",
        "id", "name", "expiration date"
    );
}

fn print_certificate(cert: &Certificate) {
    let expiration_date = cert.attributes.expiration_date.split_once('T').unwrap().0;
    println!(
        "{: <10} | {: <50} | {: <10}",
        cert.id, cert.attributes.name, expiration_date
    );
}

#[derive(Subcommand)]
enum DeviceCommand {
    Register {
        /// Path to unified api key.
        #[clap(long)]
        api_key: PathBuf,
        /// Name for device.
        #[clap(long)]
        name: String,
        /// Platform.
        #[clap(long)]
        platform: BundleIdPlatform,
        /// Unique Device Identifier
        #[clap(long)]
        udid: String,
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
        /// Id of device.
        id: String,
    },
}

impl DeviceCommand {
    fn run(self) -> Result<()> {
        match self {
            Self::Register {
                api_key,
                name,
                platform,
                udid,
            } => {
                let resp = AppStoreConnectClient::from_json_path(&api_key)?
                    .register_device(&name, platform, &udid)?;
                print_device_header();
                print_device(&resp.data);
            }
            Self::List { api_key } => {
                let resp = AppStoreConnectClient::from_json_path(&api_key)?.list_devices()?;
                print_device_header();
                for device in &resp.data {
                    print_device(device);
                }
            }
            Self::Get { api_key, id } => {
                let resp = AppStoreConnectClient::from_json_path(&api_key)?.get_device(&id)?;
                print_device_header();
                print_device(&resp.data);
            }
        }
        Ok(())
    }
}

fn print_device_header() {
    println!(
        "{: <10} | {: <20} | {: <20} | {: <20}",
        "id", "name", "model", "udid"
    );
}

fn print_device(device: &Device) {
    println!(
        "{: <10} | {: <20} | {: <20} | {: <20}",
        device.id, device.attributes.name, device.attributes.model, device.attributes.udid,
    );
}

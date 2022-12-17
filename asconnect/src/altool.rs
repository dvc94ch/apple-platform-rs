use crate::AppStoreConnectClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

const JSON_RPC_URL: &'static str =
    "https://contentdelivery.itunes.apple.com/WebObjects/MZLabelService.woa/json";

impl AppStoreConnectClient {
    fn authenticate_for_session(&self) -> Result<Session> {
        let token = self.get_token()?;
        let body = json!({
            "id": "0",
            "jsonrpc": "2.0",
            "method": "authenticateForSession",
            "params": {
                "Application": "altool",
                "ApplicationBundleId": "com.apple.itunes.altool",
                "FrameworkVersions": {
                    "com.apple.itunes.connect.ITunesConnectFoundation": "4.071 (1221)",
                    "com.apple.itunes.connect.ITunesPackage": "4.071 (1221)",
                },
                "OSIdentifier": "Mac OS X 12.1.0 (x86_64)",
                "Version": "4.071 (1221)",
            }
        });
        let req = self
            .client
            .post(format!("{}/MZContentDeliveryService", JSON_RPC_URL))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("x-tx-client-version", "4.071 (1221)")
            .header("x-tx-method", "authenticateForSession")
            .header("x-tx-client-name", "altool")
            .json(&body);
        let resp: JsonRpcResult<Session> = self.send_request(req)?.json()?;
        Ok(resp.result)
    }

    fn lookup_software_for_bundle_id(
        &self,
        session: &Session,
        bundle_id: &str,
    ) -> Result<Vec<Attribute>> {
        let token = self.get_token()?;
        let body = json!({
            "id": "0",
            "jsonrpc": "2.0",
            "method": "lookupSoftwareForBundleId",
            "params": {
                "Application": "altool",
                "ApplicationBundleId": "com.apple.itunes.altool",
                "FrameworkVersions": {
                    "com.apple.itunes.connect.ITunesConnectFoundation": "4.071 (1221)",
                    "com.apple.itunes.connect.ITunesPackage": "4.071 (1221)",
                },
                "OSIdentifier": "Mac OS X 12.1.0 (x86_64)",
                "Version": "4.071 (1221)",
                "BundleId": bundle_id,
            }
        });
        let req = self
            .client
            .post(format!("{}/MZITunesSoftwareService", JSON_RPC_URL))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("x-session-id", &session.session_id)
            .header("x-session-digest", session.digest(&body))
            .header("x-tx-client-version", "4.071 (1221)")
            .header("x-tx-method", "lookupSoftwareForBundleId")
            .header("x-tx-client-name", "altool")
            .json(&body);
        let resp: JsonRpcResult<Attributes> = self.send_request(req)?.json()?;
        Ok(resp.result.attributes)
    }

    fn prepare_session(&self, path: &Path) -> Result<(Session, String)> {
        let bundle_id = extract_bundle_id(path)?;
        let session = self.authenticate_for_session()?;
        let attributes = self.lookup_software_for_bundle_id(&session, &bundle_id)?;
        let attribute = attributes
            .into_iter()
            .find(|attr| attr.r#type == "iOS App" && attr.software_type_enum == "Purple")
            .context("failed to find app")?;
        Ok((session, attribute.apple_id))
    }

    pub fn validate(&self, path: &Path) -> Result<()> {
        let (_session, apple_id) = self.prepare_session(path)?;
        println!("{}", apple_id);
        Ok(())
    }

    pub fn upload(&self, path: &Path) -> Result<()> {
        let (_session, _apple_id) = self.prepare_session(path)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcResult<T> {
    pub result: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Session {
    pub session_id: String,
    pub shared_secret: String,
}

impl Session {
    fn digest(&self, request: &Value) -> String {
        let preimage = format!("{}{}{}", self.session_id, request, self.shared_secret);
        let digest = md5::compute(preimage.as_bytes());
        format!("{:x}", digest)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Attributes {
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Attribute {
    #[serde(rename = "AppleID")]
    pub apple_id: String,
    pub r#type: String,
    pub software_type_enum: String,
}

fn extract_bundle_id(path: &Path) -> Result<String> {
    let name = path.file_stem().unwrap().to_str().unwrap();
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let info = archive.by_name(&format!("Payload/{}.app/Info.plist", name))?;
    let info: plist::Value = plist::from_reader_xml(info)?;
    let bundle_identifier = info
        .as_dictionary()
        .context("invalid Info.plist")?
        .get("CFBundleIdentifier")
        .context("invalid Info.plist")?
        .as_string()
        .context("invalid Info.plist")?;
    Ok(bundle_identifier.to_string())
}

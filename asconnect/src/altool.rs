use crate::AppStoreConnectClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

const JSON_RPC_URL: &'static str =
    "https://contentdelivery.itunes.apple.com/WebObjects/MZLabelService.woa/json";

impl AppStoreConnectClient {
    fn lookup_software_for_bundle_id(
        &self,
        bundle_id: &str,
    ) -> Result<Vec<Attribute>> {
        let token = self.get_token()?;
        let body = json!({
            "id": "0",
            "jsonrpc": "2.0",
            "method": "lookupSoftwareForBundleId",
            "params": {
                "BundleId": bundle_id,
            }
        });
        let req = self
            .client
            .post(format!("{}/MZITunesSoftwareService", JSON_RPC_URL))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body);
        let resp: JsonRpcResult<Attributes> = self.send_request(req)?.json()?;
        Ok(resp.result.attributes)
    }

    fn prepare_session(&self, path: &Path) -> Result<String> {
        let bundle_id = extract_bundle_id(path)?;
        let attributes = self.lookup_software_for_bundle_id(&bundle_id)?;
        let attribute = attributes
            .into_iter()
            .find(|attr| attr.r#type == "iOS App" && attr.software_type_enum == "Purple")
            .context("failed to find app")?;
        Ok(attribute.apple_id)
    }

    pub fn validate(&self, path: &Path) -> Result<()> {
        let apple_id = self.prepare_session(path)?;
        println!("{}", apple_id);
        Ok(())
    }

    pub fn upload(&self, path: &Path) -> Result<()> {
        let apple_id = self.prepare_session(path)?;
        println!("{}", apple_id);
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcResult<T> {
    pub result: T,
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

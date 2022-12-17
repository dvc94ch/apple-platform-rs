use crate::AppStoreConnectClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

const DOMAIN: &'static str = "https://contentdelivery.itunes.apple.com";
const JSON_RPC: &'static str = "/WebObjects/MZLabelService.woa/json";
const IRIS: &'static str = "/MZContentDeliveryService/iris/v1";

impl AppStoreConnectClient {
    fn lookup_software_for_bundle_id(&self, bundle_id: &str) -> Result<Vec<Attribute>> {
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
            .post(format!("{}{}/MZITunesSoftwareService", DOMAIN, JSON_RPC))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body);
        let resp: JsonRpcResult<Attributes> = self.send_request(req)?.json()?;
        Ok(resp.result.attributes)
    }

    fn create_build(&self, id: &str, version: &str, short_version_string: &str) -> Result<String> {
        let token = self.get_token()?;
        let body = json!({
            "data": {
                "attributes": {
                    "cfBundleShortVersionString": short_version_string,
                    "cfBundleVersion": version,
                    "platform": "IOS",
                },
                "relationships": {
                    "app": {
                        "data": {
                            "id": id,
                            "type": "apps",
                        }
                    }
                },
                "type": "builds"
            }
        });
        let req = self
            .client
            .post(format!("{}{}/builds", DOMAIN, IRIS))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body);
        let resp: Data = self.send_request(req)?.json()?;
        Ok(resp.data.id)
    }

    pub fn upload(&self, path: &Path) -> Result<()> {
        let app_data = extract_app_data(path)?;
        let attributes = self.lookup_software_for_bundle_id(&app_data.cf_bundle_identifier)?;
        let attribute = attributes
            .into_iter()
            .find(|attr| attr.r#type == "iOS App" && attr.software_type_enum == "Purple")
            .context("failed to find app")?;
        let apple_id = attribute.apple_id;
        let build_id = self.create_build(
            &apple_id,
            &app_data.cf_bundle_version,
            &app_data.cf_bundle_short_version_string,
        )?;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    data: BuildId,
}

#[derive(Debug, Deserialize)]
pub struct BuildId {
    id: String,
}

fn extract_app_data(path: &Path) -> Result<AppData> {
    let name = path.file_stem().unwrap().to_str().unwrap();
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let info = archive.by_name(&format!("Payload/{}.app/Info.plist", name))?;
    let info: plist::Value = plist::from_reader_xml(info)?;
    let info = info.as_dictionary().context("invalid Info.plist")?;
    fn get_string(dict: &plist::Dictionary, key: &str) -> Result<String> {
        Ok(dict
            .get(key)
            .context("invalid Info.plist")?
            .as_string()
            .context("invalid Info.plist")?
            .to_string())
    }
    let cf_bundle_identifier = get_string(info, "CFBundleIdentifier")?;
    let cf_bundle_version = get_string(info, "CFBundleVersion")?;
    let cf_bundle_short_version_string = get_string(info, "CFBundleShortVersionString")?;
    Ok(AppData {
        cf_bundle_identifier,
        cf_bundle_version,
        cf_bundle_short_version_string,
    })
}

struct AppData {
    cf_bundle_identifier: String,
    cf_bundle_version: String,
    cf_bundle_short_version_string: String,
}

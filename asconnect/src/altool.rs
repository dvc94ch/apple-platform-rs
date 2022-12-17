use crate::AppStoreConnectClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
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
        let resp: CreateBuildResponse = self.send_request(req)?.json()?;
        Ok(resp.data.id)
    }

    fn create_upload(&self, build_id: &str, path: &Path) -> Result<()> {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let mut f = File::open(path)?;
        let file_size = f.metadata()?.len();
        let mut data = Vec::with_capacity(file_size as _);
        f.read_to_end(&mut data)?;
        let digest = md5::compute(&data);
        let file_checksum = format!("{:x}", digest);

        let token = self.get_token()?;
        let body = json!({
            "data": {
                "attributes": {
                    "assetType": "ASSET_DESCRIPTION",
                    "fileName": file_name,
                    "fileSize": file_size,
                    "sourceFileChecksum": file_checksum,
                    "uti": "public.binary",
                },
                "relationships": {
                    "build": {
                        "data": {
                            "id": build_id,
                            "type": "builds",
                        }
                    }
                },
                "type": "buildDeliveryFiles"
            }
        });
        let req = self
            .client
            .post(format!("{}{}/buildDeliveryFiles", DOMAIN, IRIS))
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body);
        let resp: CreateBuildDeliveryResponse = self.send_request(req)?.json()?;
        let id = resp.data.id;
        let operations = resp.data.attributes.upload_operations;

        for operation in operations {
            let mut buf = Vec::with_capacity(operation.length as _);
            f.seek(SeekFrom::Start(operation.offset))?;
            (&mut f).take(operation.length).read_to_end(&mut buf)?;
            let req = self.client.put(&operation.url).body(buf);
            self.send_request(req)?;
        }

        let body = json!({
            "data": {
                "attributes": {
                    "uploaded": true
                },
                "id": id,
                "type": "buildDeliveryFiles",
            },
        });
        let req = self
            .client
            .patch(format!("{}{}/buildDeliveryFiles", DOMAIN, IRIS))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body);
        self.send_request(req)?;
        Ok(())
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
        self.create_upload(&build_id, path)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcResult<T> {
    pub result: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Attributes {
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Attribute {
    #[serde(rename = "AppleID")]
    pub apple_id: String,
    pub r#type: String,
    pub software_type_enum: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBuildResponse {
    pub data: BuildData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildData {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBuildDeliveryResponse {
    pub data: BuildDeliveryData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildDeliveryData {
    pub attributes: BuildDeliveryAttributes,
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildDeliveryAttributes {
    pub upload_operations: Vec<UploadOperation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadOperation {
    pub offset: u64,
    pub length: u64,
    pub url: String,
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

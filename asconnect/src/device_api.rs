use crate::AppStoreConnectClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const APPLE_CERTIFICATE_URL: &'static str = "https://api.appstoreconnect.apple.com/v1/devices";

impl AppStoreConnectClient {
    pub fn register_device(
        &self,
        name: &str,
        platform: BundleIdPlatform,
        udid: &str,
    ) -> Result<DeviceResponse> {
        let token = self.get_token()?;
        let body = DeviceCreateRequest {
            data: DeviceCreateRequestData {
                attributes: DeviceCreateRequestAttributes {
                    name: name.into(),
                    platform: platform.to_string(),
                    udid: udid.into(),
                },
                r#type: "devices".into(),
            },
        };
        let req = self
            .client
            .post(APPLE_CERTIFICATE_URL)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body);
        Ok(self.send_request(req)?.json()?)
    }

    pub fn list_devices(&self) -> Result<DevicesResponse> {
        let token = self.get_token()?;
        let req = self
            .client
            .get(APPLE_CERTIFICATE_URL)
            .bearer_auth(token)
            .header("Accept", "application/json");
        Ok(self.send_request(req)?.json()?)
    }

    pub fn get_device(&self, id: &str) -> Result<DeviceResponse> {
        let token = self.get_token()?;
        let req = self
            .client
            .get(format!("{}/{}", APPLE_CERTIFICATE_URL, id))
            .bearer_auth(token)
            .header("Accept", "application/json");
        Ok(self.send_request(req)?.json()?)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCreateRequest {
    pub data: DeviceCreateRequestData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCreateRequestData {
    pub attributes: DeviceCreateRequestAttributes,
    pub r#type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCreateRequestAttributes {
    pub name: String,
    pub platform: String,
    pub udid: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleIdPlatform {
    Ios,
    MacOs,
}

impl std::fmt::Display for BundleIdPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Self::Ios => "IOS",
            Self::MacOs => "MAC_OS",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for BundleIdPlatform {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "ios" => Self::Ios,
            "macos" => Self::MacOs,
            _ => anyhow::bail!("unsupported bundle id platform {}", s),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceResponse {
    pub data: Device,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicesResponse {
    pub data: Vec<Device>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub attributes: DeviceAttributes,
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAttributes {
    pub device_class: String,
    pub model: String,
    pub name: String,
    pub platform: String,
    pub status: String,
    pub udid: String,
    pub added_date: String,
}

use crate::AppStoreConnectClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const APPLE_CERTIFICATE_URL: &'static str = "https://api.appstoreconnect.apple.com/v1/profiles";

impl AppStoreConnectClient {
    pub fn create_profile(
        &self,
        name: &str,
        profile_type: ProfileType,
        bundle_id: &str,
    ) -> Result<ProfileResponse> {
        let token = self.get_token()?;
        let body = ProfileCreateRequest {
            data: ProfileCreateRequestData {
                attributes: ProfileCreateRequestAttributes {
                    name: name.into(),
                    profile_type: profile_type.to_string(),
                },
                relationships: ProfileCreateRequestRelationships {
                    bundle_id: BundleId {
                        data: BundleIdData {
                            id: bundle_id.into(),
                            r#type: "bundleIds".into(),
                        },
                    },
                    // TODO
                    certificates: vec![],
                    // TODO
                    devices: vec![],
                },
                r#type: "profiles".into(),
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

    pub fn list_profiles(&self) -> Result<ProfilesResponse> {
        let token = self.get_token()?;
        let req = self
            .client
            .get(APPLE_CERTIFICATE_URL)
            .bearer_auth(token)
            .header("Accept", "application/json");
        Ok(self.send_request(req)?.json()?)
    }

    pub fn get_profile(&self, id: &str) -> Result<ProfileResponse> {
        let token = self.get_token()?;
        let req = self
            .client
            .get(format!("{}/{}", APPLE_CERTIFICATE_URL, id))
            .bearer_auth(token)
            .header("Accept", "application/json");
        Ok(self.send_request(req)?.json()?)
    }

    pub fn delete_profile(&self, id: &str) -> Result<()> {
        let token = self.get_token()?;
        let req = self
            .client
            .delete(format!("{}/{}", APPLE_CERTIFICATE_URL, id))
            .bearer_auth(token);
        self.send_request(req)?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateRequest {
    pub data: ProfileCreateRequestData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateRequestData {
    pub attributes: ProfileCreateRequestAttributes,
    pub relationships: ProfileCreateRequestRelationships,
    pub r#type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateRequestAttributes {
    pub name: String,
    pub profile_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateRequestRelationships {
    pub bundle_id: BundleId,
    pub certificates: Vec<()>,
    pub devices: Vec<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleId {
    pub data: BundleIdData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleIdData {
    pub id: String,
    pub r#type: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileType {
    IosAppDevelopment,
    MacAppDevelopment,
    IosAppStore,
    MacAppStore,
    MacAppDirect,
}

impl std::fmt::Display for ProfileType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Self::IosAppDevelopment => "IOS_APP_DEVELOPMENT",
            Self::MacAppDevelopment => "MAC_APP_DEVELOPMENT",
            Self::IosAppStore => "IOS_APP_STORE",
            Self::MacAppStore => "MAC_APP_STORE",
            Self::MacAppDirect => "MAC_APP_DIRECT",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ProfileType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "ios-dev" => Self::IosAppDevelopment,
            "macos-dev" => Self::MacAppDevelopment,
            "ios-appstore" => Self::IosAppStore,
            "macos-appstore" => Self::MacAppStore,
            "notarization" => Self::MacAppDirect,
            _ => anyhow::bail!("unsupported bundle id platform {}", s),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    pub data: Profile,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesResponse {
    pub data: Vec<Profile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub attributes: ProfileAttributes,
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAttributes {
    pub name: String,
    pub platform: String,
    pub profile_content: String,
    pub uuid: String,
    pub created_date: String,
    pub profile_state: String,
    pub profile_type: String,
    pub expiration_date: String,
}

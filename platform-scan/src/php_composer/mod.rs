use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerLockPackage {
    pub name: String,
    pub version: String,

    #[serde(rename = "type")]
    pub package_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerLock {
    // content-hash is not interessting
    pub packages: Vec<ComposerLockPackage>,
    // additionel fields might be relevant like platform: HashMap<String, String>
}

impl ComposerLock {}

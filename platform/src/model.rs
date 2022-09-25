use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Oauth2 {
    pub access_token: String,
    pub expires_in: i32,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HALLink {
    pub title: Option<String>,
    pub href: String,
    pub method: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub owner_id: String, // UUID
    pub namespace: String,
    pub name: String,
    pub label: String,
    pub country: String,
    pub created_at: Option<DateTime<Local>>, // date-time
    pub updated_at: Option<DateTime<Local>>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Organizations {
    // pub count: i32,
    pub items: Vec<Organization>,
    pub _links: HashMap<String,HALLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub status: String,
    pub created_at: String, // date-time something chrono?
    pub owner: String,      // UUID
    // pub owner_info struct
    // pub vendor: String, 
    pub plan: String,
    pub environments: i32,
    pub storage: i32, // in MiB
    // pub user_licenses: i32,
    pub project_id: String,
    // pub project_endpoint: String, // doesn't seem to exist
    pub project_title: String,
    pub project_region: Option<String>,
    pub project_region_label: Option<String>,
    // pub project_notes: String, // not set
    pub project_ui: String, // URL
    // pub project_options: struct...
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscriptions {
    // pub count: i32,
    pub items: Vec<Subscription>,
    pub _links: HashMap<String,HALLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub created_at: Option<DateTime<Local>>, // date-time
    pub updated_at: Option<DateTime<Local>>, // date-time

    pub name: String,
    pub machine_name: String,
    pub title: String,
    pub edge_hostname: String,

    pub attributes: HashMap<String, String>,

    #[serde(rename = "type")]
    pub e_type: String,

    pub parent: Option<String>,
    pub clone_parent_on_create: bool,
    pub deployment_target: String,
    pub status: String, // enum "active" "dirty" "inactive" "deleting"

    pub is_dirty: bool,
    pub is_main: bool,
    pub is_pr: bool,
    pub has_code: bool,
    pub has_deployment: bool,

    pub last_backup_at: Option<DateTime<Local>>,
    pub last_active_at: Option<DateTime<Local>>,

    pub head_commit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformAppCron {
    pub spec: String,
    pub cmd: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformApp {
    pub name: String,
    #[serde(rename="type")]
    pub a_type: String,
    pub build: Option<HashMap<String, String>>,
    pub hooks: Option<HashMap<String, String>>,
    pub crons: HashMap<String, PlatformAppCron>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformService {
    #[serde(rename="type")]
    pub s_type: String,
    pub disk: Option<i32>,
    pub size: Option<String>,
    // configuration: Option<HashMap<String, String>>, // more complex
    pub relationships: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCommit {
    pub id: String,
    pub sha: String,
    pub tree: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitTreeItem {
    pub path: String,
    pub mode: String,
    #[serde(rename="type")]
    pub t_type: String,
    pub sha: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitTree {
    pub id: String,
    pub tree: Vec<GitTreeItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitBlob {
    pub sha: String,
    pub size: u32,
    pub encoding: String,
    pub content: String
}

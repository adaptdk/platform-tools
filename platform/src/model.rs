use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub updated_at: Option<DateTime<Local>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Organizations {
    // pub count: i32,
    pub items: Vec<Organization>,
    pub _links: HashMap<String, HALLink>,
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
    pub _links: HashMap<String, HALLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,

    pub attributes: HashMap<String, String>,

    pub title: String,
    pub description: String,

    pub namespace: String,
    pub organization: String,
    pub default_branch: Option<String>,

    // should be struct { code: ..., message: ... }
    pub status: HashMap<String, String>,

    pub timezone: String,
    pub region: String,

    // should be struct { url: ..., client_ssh_key: ... }
    pub repository: HashMap<String, String>,

    pub default_domain: Option<String>,

    // pub subscription: ...
    pub _links: HashMap<String, HALLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Projects {
    pub count: i32,
    pub items: Vec<Project>,
    pub _links: HashMap<String, HALLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Environment {
    pub created_at: Option<DateTime<Local>>, // date-time
    pub updated_at: Option<DateTime<Local>>, // date-time

    pub name: String,
    pub machine_name: String,
    pub title: String,
    pub edge_hostname: String,

    pub attributes: HashMap<String, String>,

    #[serde(rename = "type")]
    pub r#type: String,

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
pub struct PlatformAppCronCommands {
    pub start: String,
    pub stop: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformAppCron {
    pub spec: String,

    pub cmd: Option<String>, // deprecated it seems
    pub commands: Option<PlatformAppCronCommands>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformApp {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub build: Option<HashMap<String, String>>,
    pub hooks: Option<HashMap<String, String>>,
    pub crons: Option<HashMap<String, PlatformAppCron>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformService {
    #[serde(rename = "type")]
    pub r#type: String,
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
    #[serde(rename = "type")]
    pub r#type: String,
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
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Activity {
    pub id: String, // not in spec
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub parameters: HashMap<String, Value>,
    pub state: String,
    pub result: Option<String>,
    pub started_at: Option<DateTime<Local>>,
    pub completed_at: Option<DateTime<Local>>,
    pub cancelled_at: Option<DateTime<Local>>,
    pub timings: HashMap<String, f64>,
    pub _links: HashMap<String, HALLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String, // UUID
    pub deactivated: bool,
    pub namespace: String,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub first_name: String,
    pub last_name: String,
    pub picture: String,
    pub company: String,
    pub website: String,
    pub country: String,
    // pub mfa_enabled: bool,
    pub phone_number_verified: bool,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub attributes: HashMap<String, Value>,
    pub value: Option<String>,
    pub is_json: bool,
    pub is_sensitive: bool,
    pub visible_build: bool,
    pub visible_runtime: bool,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub attributes: HashMap<String, Value>,
    pub value: Option<String>,
    pub is_json: bool,
    pub is_sensitive: bool,
    pub visible_build: bool,
    pub visible_runtime: bool,
    pub project: String,
    pub environment: String,
    pub inherited: bool,
    pub is_enabled: bool,
    pub is_inheritable: bool,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

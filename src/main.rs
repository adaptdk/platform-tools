use std::{collections::HashMap, env, str};
use reqwest;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

mod platform;

#[derive(Debug, Serialize, Deserialize)]
struct Oauth2 {
    access_token: String,
    expires_in: i32,
    token_type: String
}

#[derive(Debug, Serialize, Deserialize)]
struct Subscription {
    id: String,
    status: String,
    created_at: String, // date-time something chrono?
    owner: String, // UUID
    // owner_info struct
    // vendor: String, 
    plan: String,
    environments: i32,
    storage: i32, // in MiB
    // user_licenses: i32,
    project_id: String,
    // project_endpoint: String, // doesn't seem to exist
    project_title: String,
    project_region: Option<String>,
    project_region_label: Option<String>,
    // project_notes: String, // not set
    project_ui: String, // URL
    // project_options: struct...
}

#[derive(Debug, Serialize, Deserialize)]
struct Subscriptions {
    count: i32,
    subscriptions: Vec<Subscription>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Environment {
    created_at: Option<DateTime<Local>>, // date-time
    updated_at: Option<DateTime<Local>>, // date-time

    name: String,
    machine_name: String,
    title: String,
    edge_hostname: String,

    attributes: HashMap<String, String>,

    #[serde(rename="type")]
    e_type: String,

    parent: Option<String>,
    clone_parent_on_create: bool,
    deployment_target: String,
    status: String, // enum "active" "dirty" "inactive" "deleting"

    is_dirty: bool,
    is_main: bool,
    is_pr: bool,
    has_code: bool,
    has_deployment: bool,

    last_backup_at: Option<String>,
    last_active_at: Option<String>,

    head_commit: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct PlatformAppCron {
    spec: String,
    cmd: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlatformApp {
    name: String,
    #[serde(rename="type")]
    a_type: String,
    crons: HashMap<String, PlatformAppCron>,
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let token = env::var("PLATFORMSH_CLI_TOKEN").expect("Missing PLATFORMSH_CLI-TOKEN");

    let client = platform::ApiClient::new(&token).await?;

    eprint!("get subscriptions... ");
    let subscriptions: Subscriptions = client
        .get("https://api.platform.sh/subscriptions")
        .send()
        .await?
        .json()
        .await?;
    eprintln!("ok {}", subscriptions.count);

    for subscription in subscriptions.subscriptions.iter() {
        println!("{},{},{},{}", subscription.project_id, subscription.project_title, subscription.plan, subscription.storage);
        if subscription.project_id != "botcwpoam2wde" { 
            continue;
        }
    
        let environments: Vec<Environment> = client
            .get(format!("https://api.platform.sh/projects/{}/environments", subscription.project_id))
            .send()
            .await?
            .json()
            .await?;

        for environment in environments.iter() {
            if environment.is_main {
                if let Some(head_commit) = environment.head_commit.as_ref() {
                    let git_commit = client
                        .git_commit(&subscription.project_id, head_commit)
                        .await?;
                    println!("{:#?}", git_commit);
                    
                    // let git_tree: platform::GitTree = client
                    //     .git_tree(&subscription.project_id, &git_commit.tree)
                    //     .await?;
                    // println!("{:#?}", git_tree);

                    let items = client
                        .git_tree_find(
                            &subscription.project_id, 
                            &git_commit.tree, 
                            |path| { path == ".platform.app.yaml" || path == "composer.lock"},
                             2
                        )
                        .await?;
                    println!("{:#?}", items);

                    for item in items.iter().filter(|x| x.path == ".platform.app.yaml") {
                        let blob = client
                            .git_blob(&subscription.project_id, &item.sha)
                            .await?;
                        if let Ok(content) = &base64::decode(blob.content) {
                            if let Ok(app) = serde_yaml::from_slice::<PlatformApp>(content) {
                                println!("{:#?}", app);

                                if app.a_type.starts_with("php:") {
                                    for lock in items.iter().filter(|x| x.path == "composer.lock" && x.parent == item.parent) {
                                        println!("got {}", lock.path);
                                    }
                                }
                            }
                        }

                    }

                }
            }
        }
        break;
    }

    Ok(())
}


// #[allow(dead_code)]
// async fn default_branch (projectId: String) -> Option<String> {
//     /*
//     get all list of environments
//     find the one whit environment.is_main
//     https://github.com/platformsh/platformsh-cli/blob/f6f3777efe0b9c64bcb36d19171de0e07247e43b/src/Service/Api.php#L1168
//     */

//     let mut default_branch: Option<String> = None;

//     default_branch
// }

// Replicate getTree https://github.com/platformsh/platformsh-cli/blob/179acf1e4b2312dee136be0785492b09d3de6973/src/Service/GitDataApi.php#L200
// to do `git ls-tree | grep .platform.app.yaml`
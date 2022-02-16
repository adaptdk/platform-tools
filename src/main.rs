use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize}; // , de::value
// use tokio::io::AsyncWriteExt;
use std::{collections::HashMap, env, fs::File, io, str};

mod php_composer;
mod platform;

#[derive(Debug, Serialize, Deserialize)]
struct Environment {
    created_at: Option<DateTime<Local>>, // date-time
    updated_at: Option<DateTime<Local>>, // date-time

    name: String,
    machine_name: String,
    title: String,
    edge_hostname: String,

    attributes: HashMap<String, String>,

    #[serde(rename = "type")]
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

    last_backup_at: Option<DateTime<Local>>,
    last_active_at: Option<DateTime<Local>>,

    head_commit: Option<String>,
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
    build: Option<HashMap<String, String>>,
    hooks: Option<HashMap<String, String>>,
    crons: HashMap<String, PlatformAppCron>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    frameworks: Option<HashMap<String, Vec<String>>>,
    packages: Option<Vec<String>>,
}

impl Config {
    fn packages_map(&self) -> HashMap<String, String> {
        let mut map: HashMap<String, String> = HashMap::new();

        if let Some(packages) = &self.packages {
            for name in packages.iter() {
                map.insert(name.to_string(), name.to_string());
            }
        }

        // Seems very unelegant
        if let Some(frameworks) = &self.frameworks {
            for (framework, aliases) in frameworks.iter() {
                for alias in aliases.iter() {
                    map.insert(alias.to_string(), framework.to_string());
                }
            }
        }

        map
    }

    fn report_cols(&self) -> Vec<String> {
        let mut cols: Vec<String> = Vec::new();

        if let Some(frameworks) = &self.frameworks {
            for (framework, _aliases) in frameworks.iter() {
                cols.push(framework.to_string());
            }
        }

        if let Some(packages) = &self.packages {
            for name in packages.iter() {
                cols.push(name.to_string());
            }
        }

        cols
    }
}

#[derive(Debug, Serialize, Clone)]
struct Report {
    // Subscription
    subscription: String,
    title: String,
    plan: String,
    storage: i32,
 
    // Environment
    last_backup_at: Option<DateTime<Local>>,
    
    // App
    a_type: String,
    app: String,
    packages: HashMap<String,String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("PLATFORMSH_CLI_TOKEN").expect("Missing PLATFORMSH_CLI_TOKEN");

    let file = File::open("config.yaml")?;
    let config: Config = serde_yaml::from_reader(file)?;
    let packages_map = config.packages_map();
    let mut lines: Vec<Report> = Vec::new();

    
    let client = platform::ApiClient::new(&token).await?;

    let subscriptions = client.subscriptions().await?;
    for subscription in subscriptions.iter() {
        eprintln!("{},{},{},{}", subscription.project_id, subscription.project_title, subscription.plan, subscription.storage);
        // if subscription.project_id != "botcwpoam2wde" && subscription.project_id != "zmkzkflclscto" { 
        //    continue;
        // }
    
        let environments_res: Result<Vec<Environment>, reqwest::Error> = client
            .get(format!(
                "https://api.platform.sh/projects/{}/environments", 
                subscription.project_id
            ))
            .send()
            .await?
            .json()
            .await;
        
        if let Ok(environments) = environments_res {
            for environment in environments.iter() {
                eprintln!("\t{}: {}", environment.title, environment.is_main);
                if environment.is_main {
                    eprintln!("{:#?}", environment);
                    eprintln!("\t{}", environment.name);
                    if let Some(head_commit) = environment.head_commit.as_ref() {
                        let git_commit = client
                            .git_commit(&subscription.project_id, head_commit)
                            .await?;
                        eprintln!("{:#?}", git_commit);
                        
                        let items = client
                            .git_tree_find(
                                &subscription.project_id, 
                                &git_commit.tree, 
                                |path| path == ".platform.app.yaml" || path == "composer.lock",
                                 2
                            )
                            .await?;
                        // eprintln!("{:#?}", items);
    
                        for item in items.iter().filter(|x| x.path == ".platform.app.yaml") {
                            let blob = client
                                .git_blob(&subscription.project_id, &item.sha)
                                .await?;
                            if let Ok(content) = &base64::decode(blob.content) {
                                if let Ok(app) = serde_yaml::from_slice::<PlatformApp>(content) {
                                    // eprintln!("{:#?}", app);
                                    eprintln!("\t\t{}", app.name);
    
                                    let mut version = HashMap::new();
                                    if app.a_type.starts_with("php:") {
                                        for lock in items.iter().filter(|x| x.path == "composer.lock" && x.parent == item.parent) {
                                            // eprintln!("got {} {}", app.name, lock.path);
    
                                            if let Ok(buffer) = client.git_blob_decode(&subscription.project_id, &lock.sha).await {
                                                if let Ok(composer_lock) = serde_json::from_slice::<php_composer::ComposerLock>(&buffer) {
                                                    for package in &composer_lock.packages {
                                                        if let Some(name) = packages_map.get(&package.name) {
                                                            version.insert(name.to_string(), package.version.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
    
                                    let report = Report {
                                        subscription: subscription.project_id.to_string(),
                                        title: subscription.project_title.to_string(),
                                        plan: subscription.plan.to_string(),
                                        storage: subscription.storage,
    
                                        last_backup_at: environment.last_backup_at,
    
                                        app: app.name.to_string(),
                                        a_type: app.a_type.to_string(),
                                        packages: version,
                                    };
    
                                    lines.push(report);
                                }
                            }
                        }
                    } else {
                        eprintln!("no head commit");
                    }
                }
            }
        } else {
            let report = Report {
                subscription: subscription.project_id.to_string(),
                title: subscription.project_title.to_string(),
                plan: subscription.plan.to_string(),
                storage: subscription.storage,

                last_backup_at: None,

                a_type: "".to_string(),
                app: "".to_string(),
                packages: HashMap::new(),
            };
            lines.push(report);
        }
        // break;
    }

    let mut heading = vec![
        "subscription".to_string(),
        "title".to_string(),
        "plan".to_string(),
        "storage".to_string(),
        "last_backup_at".to_string(),
        "type".to_string(),
        "app".to_string(),
    ]; 
    let mut packages_cols = config.report_cols();
    heading.append(&mut packages_cols);

    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(heading)?;

    let report_cols = config.report_cols();
    lines.sort_by_cached_key(|x| -> String {format!("{}-{}", x.title, x.app)} );
    for line in lines.iter() {
        // eprintln!("{:#?}", line);
        let mut record = vec![
            line.subscription.clone(),
            line.title.clone(),
            line.plan.clone(),
            line.storage.to_string(),
            // "".to_string(),
            match line.last_backup_at {
                Some(dt) => dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
                None => "".to_string(),
            },
            line.a_type.clone(),
            line.app.clone(),
        ];
        for i in report_cols.iter() {
            record.push(
                match line.packages.get(i) {
                    Some(value) => value.clone(),
                    None => "".to_string(),
                }
            )
        }
        wtr.write_record(&record)?;
    }

    wtr.flush()?;

    // println!("{:#?}", lines);
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

use chrono::{DateTime, Local};
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io, str};
use tracing::info;

mod php_composer;

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
    packages: HashMap<String, String>,
    services: HashMap<String, String>,
}

#[derive(Parser, Debug)]
struct Args {
    /// List services
    #[arg(long, short, action)]
    services: bool,

    /// Project ID
    #[arg(long, short)]
    project: Vec<String>,

    /// Platform Access Token
    #[arg(long, env = "PLATFORMSH_CLI_TOKEN")]
    token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("{:#?}", args);

    // let token = env::var("PLATFORMSH_CLI_TOKEN").expect("Missing PLATFORMSH_CLI_TOKEN");

    tracing_subscriber::fmt::init();

    let file = File::open("config.yaml")?;
    let config: Config = serde_yaml::from_reader(file)?;
    let packages_map = config.packages_map();
    let mut lines: Vec<Report> = Vec::new();

    let drupal = Regex::new(r"projects\[drupal\]\[version\]\s*=\s*([0-9.]+)").unwrap();

    let client = platform::ApiClient::new(&args.token).await?;

    // let organizations = client.organizations().await?;
    // eprint!("{:#?}", organizations);

    let mut services_cnt = HashMap::new();
    let mut unreadable = Vec::new();
    let subscriptions = client.subscriptions().await?;
    // for subscription in subscriptions.iter().filter(|x| x.project_id == "muvtvqjnckbp6") {
    for subscription in subscriptions.iter() {
        if !args.project.is_empty() && !args.project.contains(&subscription.project_id) {
            continue;
        }

        eprintln!(
            "{},{},{},{}",
            subscription.project_id,
            subscription.project_title,
            subscription.plan,
            subscription.storage
        );
        info!(
            subscription.project_id,
            subscription.project_title, subscription.plan, subscription.storage
        );

        let environments_res: Result<Vec<platform::Environment>, reqwest::Error> = client
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
                // eprintln!("\t{}: {}", environment.title, environment.is_main);
                if environment.is_main {
                    // eprintln!("{:#?}", environment);
                    eprintln!("\t{}", environment.name);
                    if let Some(head_commit) = environment.head_commit.as_ref() {
                        let git_commit = client
                            .git_commit(&subscription.project_id, head_commit)
                            .await?;

                        let mut service_versions = HashMap::new();
                        // Check .platform/services.yaml
                        if let Some(dot_platform) = client
                            .git_tree_lookup_path(
                                &subscription.project_id,
                                &git_commit.tree,
                                ".platform",
                            )
                            .await?
                        {
                            if let Some(services_yaml) = client
                                .git_tree_lookup_path(
                                    &subscription.project_id,
                                    &dot_platform.sha,
                                    "services.yaml",
                                )
                                .await?
                            {
                                eprintln!("\t\tgot a services file {}", services_yaml.t_type);
                                if let Ok(buffer) = client
                                    .git_blob_decode(&subscription.project_id, &services_yaml.sha)
                                    .await
                                {
                                    if let Ok(services) =
                                        serde_yaml::from_slice::<
                                            HashMap<String, platform::PlatformService>,
                                        >(&buffer)
                                    {
                                        // eprintln!("{:#?}", services);
                                        for (name, service) in services.iter() {
                                            eprintln!("\t\t\t{}: {}", name, service.s_type);
                                            //if service.s_type.starts_with("elasticsearch") {
                                            //    service_versions.insert("elasticsearch", service.s_type.clone());
                                            //}
                                            if let Some((name, version)) =
                                                service.s_type.split_once(':')
                                            {
                                                service_versions
                                                    .insert(name.to_string(), version.to_string());
                                                let count = services_cnt
                                                    .entry(name.to_string())
                                                    .or_insert(0);
                                                *count += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let items = client
                            .git_tree_find(
                                &subscription.project_id,
                                &git_commit.tree,
                                |path| {
                                    path == ".platform.app.yaml"
                                        || path == "composer.lock"
                                        || path.ends_with(".make")
                                },
                                2,
                            )
                            .await?;
                        // eprintln!("{:#?}", items);

                        for item in items.iter().filter(|x| x.path == ".platform.app.yaml") {
                            let blob = client.git_blob(&subscription.project_id, &item.sha).await?;
                            if let Ok(content) = &base64::decode(blob.content) {
                                // if let Ok(app) = serde_yaml::from_slice::<platform::PlatformApp>(content) {
                                match serde_yaml::from_slice::<platform::PlatformApp>(content) {
                                    Err(error) => {
                                        eprintln!("{:#?}", error);
                                        unreadable.push(&subscription.project_id);
                                    }
                                    Ok(app) => {
                                        // eprintln!("{:#?}", app);
                                        eprintln!("\t\t{}", app.name);

                                        let mut version = HashMap::new();
                                        if app.a_type.starts_with("php:") {
                                            for lock in items.iter().filter(|x| {
                                                x.path == "composer.lock" && x.parent == item.parent
                                            }) {
                                                // eprintln!("got {} {}", app.name, lock.path);

                                                if let Ok(buffer) = client
                                                    .git_blob_decode(
                                                        &subscription.project_id,
                                                        &lock.sha,
                                                    )
                                                    .await
                                                {
                                                    if let Ok(composer_lock) =
                                                        serde_json::from_slice::<
                                                            php_composer::ComposerLock,
                                                        >(
                                                            &buffer
                                                        )
                                                    {
                                                        for package in &composer_lock.packages {
                                                            if let Some(name) =
                                                                packages_map.get(&package.name)
                                                            {
                                                                version.insert(
                                                                    name.to_string(),
                                                                    package.version.to_string(),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            if let Some(build) = app.build {
                                                if let Some(flavor) = build.get("flavor") {
                                                    if flavor == "drupal" {
                                                        // This shit is oldschool...
                                                        for make in items.iter().filter(|x| {
                                                            x.path.ends_with(".make")
                                                                && x.parent == item.parent
                                                        }) {
                                                            if let Ok(buffer) = client
                                                                .git_blob_decode(
                                                                    &subscription.project_id,
                                                                    &make.sha,
                                                                )
                                                                .await
                                                            {
                                                                if let Ok(content) =
                                                                    str::from_utf8(&buffer)
                                                                {
                                                                    for line in content.lines() {
                                                                        for cap in drupal
                                                                            .captures_iter(line)
                                                                        {
                                                                            version.insert(
                                                                                "drupal"
                                                                                    .to_string(),
                                                                                cap[1].to_string(),
                                                                            );
                                                                        }
                                                                    }
                                                                }
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
                                            services: service_versions.clone(),
                                        };

                                        lines.push(report);
                                    }
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
                services: HashMap::new(),
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

    eprintln!("{:#?}", services_cnt);
    eprintln!("Unreadable:\n{:#?}", unreadable);

    let mut services_cols: Vec<String> = services_cnt.into_keys().collect();
    if args.services {
        services_cols.sort_unstable();
        heading.append(&mut services_cols.clone());
    }

    let mut packages_cols = config.report_cols();
    heading.append(&mut packages_cols);

    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(heading)?;

    let report_cols = config.report_cols();
    lines.sort_by_cached_key(|x| -> String { format!("{}-{}", x.title.to_lowercase(), x.app) });
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

        if args.services {
            for i in services_cols.iter() {
                record.push(match line.services.get(i) {
                    Some(value) => value.clone(),
                    None => "".to_string(),
                })
            }
        }

        for i in report_cols.iter() {
            record.push(match line.packages.get(i) {
                Some(value) => value.clone(),
                None => "".to_string(),
            })
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

use reqwest::{Client, RequestBuilder, IntoUrl};
// use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
// use tokio::fs::read_to_string;
use async_recursion::async_recursion;
use thiserror::Error;
use std::{collections::HashMap, vec};

#[derive(Debug, Serialize, Deserialize)]
struct Oauth2 {
    access_token: String,
    expires_in: i32,
    token_type: String,
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
pub struct HALLink {
    pub title: Option<String>,
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscriptions {
    pub count: i32,
    pub subscriptions: Vec<Subscription>,
    pub _links: HashMap<String,HALLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitCommit {
    pub id: String,
    pub sha: String,
    pub tree: String,
}

#[derive(Debug)]
pub struct GitSearchResult {
    pub path: String,
    pub mode: String,
    pub t_type: String,
    pub sha: String,

    pub parent: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitTreeItem {
    path: String,
    mode: String,
    #[serde(rename="type")]
    t_type: String,
    sha: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitTree {
    id: String,
    tree: Vec<GitTreeItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitBlob {
    pub sha: String,
    pub size: u32,
    pub encoding: String,
    pub content: String,
}

#[derive(Debug)]
pub struct ApiClient {
    api_token: String,
    oauth2: Oauth2,
    client: Client,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
}

impl ApiClient {
    pub async fn new (api_token: &str) -> Result<ApiClient, reqwest::Error> {
        let client = reqwest::Client::new();

        eprint!("get oauth2 token... ");
        let oauth2: Oauth2 = client
            .post("https://auth.api.platform.sh/oauth2/token")
            .basic_auth( "platform-api-user", None::<String>)
            .form(&[("grant_type", "api_token"), ("api_token", api_token)])
            .send()
            .await?
            .json()
            .await?;
        eprintln!("ok");
        eprintln!("{:#?}", oauth2);

        Ok(ApiClient { api_token: api_token.to_string(), oauth2, client })
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client
            .get(url)
            .bearer_auth(&self.oauth2.access_token)
    }

    pub async fn subscriptions(&self) -> Result<Vec<Subscription>, reqwest::Error> {
        // Really ought to return a Stream/Iterator

        let mut subscriptions: Vec<Subscription> = vec![];
        let mut url = "https://api.platform.sh/subscriptions".to_string();

        eprintln!("Getting subscriptions...");
        loop {
            eprintln!("\t{}", url);
            let page: Subscriptions = self
                .get(url)
                .send()
                .await?
                .json()
                .await?;
    
            subscriptions.extend(page.subscriptions);
            
            // eprintln!("{:#?}", page._links);
            match page._links.get("next") {
                Some(next) => {
                    url = next.href.clone();
                },
                _ => { break; },
            }
        }
        
        Ok(subscriptions)
    } 

    pub async fn git_commit(&self, project_id: &str, head_commit: &str) -> Result<GitCommit, reqwest::Error> {
        // eprintln!("https://api.platform.sh/projects/{}/git/commits/{}", project_id, head_commit);
        let response = self
            .get(format!("https://api.platform.sh/projects/{}/git/commits/{}", project_id, head_commit))
            .send()
            .await?;
        // eprint!("git commit {:#?}", response);
        let git_commit: GitCommit = response
            .json()
            .await?;
        // eprintln!("ok");

        Ok(git_commit)
    }

    pub async fn git_tree(&self, project_id: &str, tree: &str) -> Result<GitTree, reqwest::Error> {
        // eprint!("https://api.platform.sh/projects/{}/git/trees/{}", project_id, tree);
        let git_tree: GitTree = self
            .get(format!("https://api.platform.sh/projects/{}/git/trees/{}", project_id, tree))
            .send()
            .await?
            .json()
            .await?;
        // println!("ok");

        Ok(git_tree)
    }

    #[async_recursion]
    pub async fn git_tree_find(&self, project_id: &str, tree: &str, f: fn(path: &str) -> bool, limit: u8) -> Result<Vec<GitSearchResult>, reqwest::Error> {
        let mut results: Vec<GitSearchResult> = Vec::new();

        if limit == 0 {
            return  Ok(results);
        }

        let git_tree = self
            .git_tree(project_id, tree)
            .await?;
        
        for item in git_tree.tree.iter() {
            if item.t_type == "tree" {
                // root format!("{}/{}", root, item.path)
                let mut sub_results = self
                    .git_tree_find(project_id, &item.sha, f, limit -1)
                    .await?;
                results.append(&mut sub_results);
            } 
            if item.t_type == "blob" && f(&item.path) {
                let res = GitSearchResult {
                    path: item.path.clone(),
                    t_type: item.t_type.clone(),
                    mode: item.mode.clone(),
                    sha: item.sha.clone(),
                    parent: String::from(tree),
                };
                results.push(res);
                // eprintln!("found {}", item.sha);
            }
        }

        Ok(results)
    }

    pub async fn git_blob(&self, project_id: &str, sha: &str) -> Result<GitBlob, reqwest::Error> {
        let git_blob: GitBlob = self
            .get(format!("https://api.platform.sh/projects/{}/git/blobs/{}", project_id, sha))
            .send()
            .await?
            .json()
            .await?;

        Ok(git_blob)
    }

    pub async fn git_blob_decode(&self, project_id: &str, sha: &str) -> Result<Vec<u8>, Error> {
        // eprintln!("download {} {}", project_id, sha);
        let blob: GitBlob = self
            .git_blob(project_id, sha)
            .await?;
        // eprintln!("download... ok");

        // Add compound Error type Enum reqwest::Error + base64 error
        let content = base64::decode(blob.content)?;
        // eprintln!("base64 decode... ok");

        Ok(content)
    }
}


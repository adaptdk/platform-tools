use reqwest::{Client, RequestBuilder, IntoUrl};
use chrono::{DateTime, Local};
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
pub struct HALLink {
    pub title: Option<String>,
    pub href: String,
    pub method: Option<String>,
}

// TODO impl TryFrom<HALLink> for Url - std::convert::TryFrom()
// impl TryFrom<HALLink> for Url {
//     type Error = ...;
//     fn try_from(value: HALLink) -> Result<Self, Self::Error> {
//     }
// }

// -- IntoUrl is sealed FFS 
// impl IntoUrl for HALLink {
//     fn into_url(self) -> Result<Url, url::Url::ParseError> {
//         let base_url = Url::parse("https://api.platform.sh").unwrap();
//         base_url.join(self.href.as_str())
//     }
// }


#[derive(Debug, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub owner_id: String, // UUID
    pub namespace: String,
    pub name: String,
    pub label: String,
    pub country: String,
    created_at: Option<DateTime<Local>>, // date-time
    updated_at: Option<DateTime<Local>>
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
    id: String,
    tree: Vec<GitTreeItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitBlob {
    pub sha: String,
    pub size: u32,
    pub encoding: String,
    pub content: String
}

#[derive(Debug)]
pub struct ApiClient {
    #[allow(dead_code)]
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

    // pub fn get(&self, url: String) -> RequestBuilder {
    //     let options = Url::options();
    //     let api = Url::parse("https://api.platform.sh");
    //     let base_url = options.base_url(Some(&api));
    //     let endpoint_url = base_url.parse(&url);

    //     self.client
    //         .get(endpoint_url)
    //         .bearer_auth(&self.oauth2.access_token)
    // }

    pub async fn organizations(&self) -> Result<Vec<Organization>, reqwest::Error> {
        // Really ought to return a Stream/Iterator

        let mut organizations: Vec<Organization> = vec![];
        let mut url = "https://api.platform.sh/organizations".to_string();

        eprintln!("Getting organizations...");
        loop {
            eprintln!("\t{}", url);
            let page: Organizations = self
                .get(url)
                .send()
                .await?
                .json()
                .await?;
    
            organizations.extend(page.items);
            
            // eprintln!("{:#?}", page._links);
            match page._links.get("next") {
                Some(next) => {
                    url = next.href.clone();
                },
                _ => { break; },
            }
        }
        
        Ok(organizations)
    }

    pub async fn subscriptions(&self) -> Result<Vec<Subscription>, reqwest::Error> {
        // Really ought to return a Stream/Iterator

        let organizations = self.organizations().await?;

        let mut subscriptions: Vec<Subscription> = vec![];
        
        for organization in organizations.iter() {
            let mut url = format!("https://api.platform.sh/organizations/{}/subscriptions", organization.id);

            eprintln!("Getting subscriptions...");
            loop {
                eprintln!("\t{}", url);
                let page: Subscriptions = self
                    .get(url)
                    .send()
                    .await?
                    .json()
                    .await?;
        
                subscriptions.extend(page.items);
                
                // eprintln!("{:#?}", page._links);
                match page._links.get("next") {
                    Some(next) => {
                        url = if next.href.starts_with("https://") {
                            next.href.clone()
                        } else {
                            format!("https://api.platform.sh{}", next.href)
                        }
                    },
                    _ => { break; },
                }
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

    pub async fn git_tree_lookup_path(&self, project_id: &str, tree: &str, path: &str) -> Result<Option<GitTreeItem>, reqwest::Error> {
        let git_tree = self
            .git_tree(project_id, tree)
            .await?;

        let mut result: Option<GitTreeItem> = None;
        for item in git_tree.tree.iter() {
            if item.path == path {
                result = Some(item.clone());
            }
        }

        Ok(result)
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


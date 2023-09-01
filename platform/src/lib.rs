use async_recursion::async_recursion;
use reqwest::{Client, RequestBuilder};
use std::vec;
use thiserror::Error;
use tracing::{info, instrument};
use url::Url;

mod model;

pub use crate::model::*;

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

#[derive(Debug)]
pub struct ApiClient {
    #[allow(dead_code)]
    api_token: String,
    oauth2: Oauth2,
    client: Client,
}

#[derive(Debug)]
pub struct GitSearchResult {
    pub path: String,
    pub mode: String,
    pub t_type: String,
    pub sha: String,

    pub parent: String,
    pub fullpath: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
    #[error("Not found")]
    NotFound
}

impl ApiClient {
    #[instrument]
    pub async fn new(api_token: &str) -> Result<ApiClient, reqwest::Error> {
        let client = reqwest::Client::new();

        // eprint!("get oauth2 token... ");
        let oauth2: Oauth2 = client
            .post("https://auth.api.platform.sh/oauth2/token")
            .basic_auth("platform-api-user", None::<String>)
            .form(&[("grant_type", "api_token"), ("api_token", api_token)])
            .send()
            .await?
            .json()
            .await?;
        // eprintln!("ok");
        // eprintln!("{:#?}", oauth2);

        Ok(ApiClient {
            api_token: api_token.to_string(),
            oauth2,
            client,
        })
    }

    #[instrument(skip(self))]
    pub fn get(&self, url: String) -> RequestBuilder {
        let options = Url::options();
        let api = Url::parse("https://api.platform.sh").unwrap();
        let base_url = options.base_url(Some(&api));
        let endpoint_url = base_url.parse(&url).unwrap();

        self.client
            .get(endpoint_url)
            .bearer_auth(&self.oauth2.access_token)
    }

    #[instrument(skip(self))]
    pub async fn organizations(&self) -> Result<Vec<Organization>, reqwest::Error> {
        // Really ought to return a Stream/Iterator

        let mut organizations: Vec<Organization> = vec![];
        let mut url = "https://api.platform.sh/organizations".to_string();

        // eprintln!("Getting organizations...");
        loop {
            // eprintln!("\t{}", url);
            info!(url);
            let page: Organizations = self.get(url).send().await?.json().await?;

            organizations.extend(page.items);

            // eprintln!("{:#?}", page._links);
            match page._links.get("next") {
                Some(next) => {
                    url = next.href.clone();
                }
                _ => {
                    break;
                }
            }
        }

        Ok(organizations)
    }

    #[instrument(skip(self))]
    pub async fn subscriptions(&self) -> Result<Vec<Subscription>, reqwest::Error> {
        // Really ought to return a Stream/Iterator

        let organizations = self.organizations().await?;

        let mut subscriptions: Vec<Subscription> = vec![];

        for organization in organizations.iter() {
            let mut url = format!(
                "https://api.platform.sh/organizations/{}/subscriptions",
                organization.id
            );

            // eprintln!("Getting subscriptions...");
            loop {
                // eprintln!("\t{}", url);
                let page: Subscriptions = self.get(url).send().await?.json().await?;

                subscriptions.extend(page.items);

                // eprintln!("{:#?}", page._links);
                match page._links.get("next") {
                    Some(next) => {
                        // url = if next.href.starts_with("https://") {
                        //     next.href.clone()
                        // } else {
                        //     format!("https://api.platform.sh{}", next.href)
                        // }
                        url = next.href.clone()
                    }
                    _ => {
                        break;
                    }
                }
            }
        }

        Ok(subscriptions)
    }

    #[instrument(skip(self))]
    pub async fn git_commit(
        &self,
        project_id: &str,
        head_commit: &str,
    ) -> Result<GitCommit, reqwest::Error> {
        // eprintln!("https://api.platform.sh/projects/{}/git/commits/{}", project_id, head_commit);
        let response = self
            .get(format!(
                "https://api.platform.sh/projects/{}/git/commits/{}",
                project_id, head_commit
            ))
            .send()
            .await?;
        // eprint!("git commit {:#?}", response);
        let git_commit: GitCommit = response.json().await?;
        // eprintln!("ok");

        Ok(git_commit)
    }

    #[instrument(skip(self))]
    pub async fn git_tree(&self, project_id: &str, tree: &str) -> Result<GitTree, reqwest::Error> {
        // eprint!("https://api.platform.sh/projects/{}/git/trees/{}", project_id, tree);
        let git_tree: GitTree = self
            .get(format!(
                "https://api.platform.sh/projects/{}/git/trees/{}",
                project_id, tree
            ))
            .send()
            .await?
            .json()
            .await?;
        // println!("ok");

        Ok(git_tree)
    }

    #[async_recursion]
    pub async fn git_tree_find(
        &self,
        project_id: &str,
        tree: &str,
        f: fn(path: &str) -> bool,
        limit: u8,
        root: String,
    ) -> Result<Vec<GitSearchResult>, reqwest::Error> {
        let mut results: Vec<GitSearchResult> = Vec::new();

        if limit == 0 {
            return Ok(results);
        }

        let git_tree = self.git_tree(project_id, tree).await?;

        for item in git_tree.tree.iter() {
            if item.t_type == "tree" {
                // root format!("{}/{}", root, item.path)
                let mut sub_results = self
                    .git_tree_find(
                        project_id,
                        &item.sha,
                        f,
                        limit - 1,
                        format!("{}/{}", root, item.path),
                    )
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
                    fullpath: format!("{}/{}", root, item.path),
                };
                results.push(res);
                // eprintln!("found {}", item.sha);
            }
        }

        Ok(results)
    }

    #[instrument(skip(self))]
    pub async fn git_tree_lookup_path(
        &self,
        project_id: &str,
        tree: &str,
        path: &str,
    ) -> Result<Option<GitTreeItem>, reqwest::Error> {
        let git_tree = self.git_tree(project_id, tree).await?;

        let mut result: Option<GitTreeItem> = None;
        for item in git_tree.tree.iter() {
            if item.path == path {
                result = Some(item.clone());
            }
        }

        Ok(result)
    }

    #[instrument(skip(self))]
    pub async fn git_blob(&self, project_id: &str, sha: &str) -> Result<GitBlob, reqwest::Error> {
        let git_blob: GitBlob = self
            .get(format!(
                "https://api.platform.sh/projects/{}/git/blobs/{}",
                project_id, sha
            ))
            .send()
            .await?
            .json()
            .await?;

        Ok(git_blob)
    }

    #[instrument(skip(self))]
    pub async fn git_blob_decode(&self, project_id: &str, sha: &str) -> Result<Vec<u8>, Error> {
        // eprintln!("download {} {}", project_id, sha);
        let blob: GitBlob = self.git_blob(project_id, sha).await?;
        // eprintln!("download... ok");

        // Add compound Error type Enum reqwest::Error + base64 error
        let content = base64::decode(blob.content)?;
        // eprintln!("base64 decode... ok");

        Ok(content)
    }

    pub async fn main_environment(&self, project_id: &str) -> Result<Environment, Error> {
        let environments_res: Result<Vec<Environment>, reqwest::Error> = self
            .get(format!(
                "https://api.platform.sh/projects/{}/environments",
                project_id
            ))
            .send()
            .await?
            .json()
            .await;

        if let Ok(environments) = environments_res {
            for environment in environments.iter() {
                if environment.is_main {
                    return Ok(environment.clone());
                }
            }
        }

        return Err(Error::NotFound)
    }
}

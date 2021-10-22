use reqwest::{Client, RequestBuilder, IntoUrl};
// use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
// use tokio::fs::read_to_string;
use async_recursion::async_recursion;

#[derive(Debug, Serialize, Deserialize)]
struct Oauth2 {
    access_token: String,
    expires_in: i32,
    token_type: String
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
    pub size: u16,
    pub encoding: String,
    pub content: String,
}

#[derive(Debug)]
pub struct ApiClient {
    api_token: String,
    oauth2: Oauth2,
    client: Client,
}

impl ApiClient {
    pub async fn new (api_token: &String) -> Result<ApiClient, reqwest::Error> {
        let client = reqwest::Client::new();

        eprint!("get oauth2 token... ");
        let oauth2: Oauth2 = client
            .post("https://auth.api.platform.sh/oauth2/token")
            .basic_auth( "platform-api-user", None::<String>)
            .form(&[("grant_type", "api_token"), ("api_token", &api_token)])
            .send()
            .await?
            .json()
            .await?;
        eprintln!("ok");
        println!("{:#?}", oauth2);

        Ok(ApiClient { api_token: api_token.to_string(), oauth2, client })
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.client
            .get(url)
            .bearer_auth(&self.oauth2.access_token)
    }

    pub async fn git_commit(&self, project_id: &str, head_commit: &str) -> Result<GitCommit, reqwest::Error> {
        eprintln!("https://api.platform.sh/projects/{}/git/commits/{}", project_id, head_commit);
        let response = self
            .get(format!("https://api.platform.sh/projects/{}/git/commits/{}", project_id, head_commit))
            .send()
            .await?;
        eprint!("git commit {:#?}", response);
        let git_commit: GitCommit = response
            .json()
            .await?;
        eprintln!("ok");

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
            if item.t_type == "blob" {
                if f(&item.path) {
                    let res = GitSearchResult {
                        path: item.path.clone(),
                        t_type: item.t_type.clone(),
                        mode: item.mode.clone(),
                        sha: item.sha.clone(),
                        parent: String::from(tree),
                    };
                    results.push(res);
                    println!("found {}", item.sha);
                }
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
}



use futures::pin_mut;
use futures::stream::{self, StreamExt};
use futures::Stream;
use std::env;
use tracing::{debug, error, info};
use platform;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("PLATFORMSH_CLI_TOKEN").expect("Missing PLATFORMSH_CLI_TOKEN");

    tracing_subscriber::fmt::init();
    let client = platform::ApiClient::new(&token).await?;
    let organizations = client.organizations().await?;
    // eprint!("{:#?}", organizations);

    for organization in organizations.iter() {

        let url = format!("https://api.platform.sh/organizations/{}/subscriptions", organization.id);
        let stream = stream::unfold(Some(url), |state| async {
            match state {
                None => None, // previous call was last page
                Some(url) => {
                    info!(url);
                    let result_buffer = client.get(url).send().await;
                    match result_buffer {
                        Ok(buffer) => {
                            let result: Result<platform::Subscriptions, reqwest::Error> = buffer.json().await;
                            match result {
                                Ok(page) => match page._links.get("next") {
                                    Some(next) => {
                                        Some((page.items, Some(next.href.clone())))
                                    },
                                    None => Some((page.items, None)), // last page
                                }
                                Err(_) => {
                                    error!("JSON Error");
                                    None
                                },
                            }
                        },
                        Err(_) => {
                            error!("Request error");
                            None
                        },
                    }
                },
            }
        })
        .flat_map_unordered(2, |page| stream::iter( page  ));

        do_subscriptions(stream).await;
    }
    
    Ok(())
}

async fn do_subscriptions (stream: impl Stream<Item=platform::Subscription>) {
    pin_mut!(stream);
    //while let Some(items) = stream.next().await {
    //     eprintln!("{:#?}", items.len());
    //}
    while let Some(item) = stream.next().await {
        eprintln!("{:#?}", item.project_title);
    }
}
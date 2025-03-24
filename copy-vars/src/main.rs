use clap::Parser;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
struct Args {
    /// Project ID
    #[arg(long, short)]
    project: String,

    #[arg(long, short)]
    destination: String,

    /// Platform Access Token
    #[arg(long, env = "PLATFORMSH_CLI_TOKEN")]
    token: String,

    /// Environment ID
    #[arg(long, short, default_value = "main")]
    environment: Vec<String>,

    /// App - which app to ssh into
    #[arg(long, short = 'A')]
    app: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let client = platform::ApiClient::new(&args.token).await?;

    let variables: Vec<platform::Variable> = client
        .get(format!(
            "https://api.platform.sh/projects/{}/variables",
            args.project
        ))
        .send()
        .await?
        .json()
        .await?;

    for v in variables {
        let value = if let Some(value) = v.value {
            value
        } else if v.visible_runtime && v.name.starts_with("env:") {
            let mut params = vec!["ssh", "-p", &args.project, "-e", &args.environment[0]];
            if let Some(ref app) = args.app {
                params.push("-A");
                params.push(app);
            }
            let echo = format!("echo -n ${}", v.name.strip_prefix("env:").unwrap());
            params.push(&echo);

            let output = Command::new("platform")
                .args(params)
                .stdout(Stdio::piped())
                .output()?;

            String::from_utf8(output.stdout)?
            //"foobar".to_string()
        } else {
            "".to_string()
        };
        if v.is_sensitive && !v.visible_runtime {
            println!("# {} must be found seperately", v.name);
            println!("# ");
        }
        println!(
            "platform variable:create --no-wait --yes --level=project --project={} --name='{}' --value='{}' --json={} --sensitive={} --visible-build={} --visible-runtime={}",
            args.destination,
            v.name,
            value,
            v.is_json,
            v.is_sensitive,
            v.visible_build,
            v.visible_runtime
        );
    }

    for environment in args.environment {
        let variables: Vec<platform::EnvironmentVariable> = client
            .get(format!(
                "https://api.platform.sh/projects/{}/environments/{}/variables",
                args.project, environment
            ))
            .send()
            .await?
            .json()
            .await?;

        for v in variables {
            if v.inherited {
                println!("# {} inherited", v.name);
                continue;
            }
            let value = if let Some(value) = v.value {
                value
            } else if v.visible_runtime && v.name.starts_with("env:") {
                let mut params = vec!["ssh", "-p", &args.project, "-e", &v.environment];
                if let Some(ref app) = args.app {
                    params.push("-A");
                    params.push(app);
                }
                let echo = format!("echo -n ${}", v.name.strip_prefix("env:").unwrap());
                params.push(&echo);

                let output = Command::new("platform")
                    .args(params)
                    .stdout(Stdio::piped())
                    .output()?;

                String::from_utf8(output.stdout)?
                //"foobar".to_string()
            } else {
                "".to_string()
            };
            // println!("# value: \"{}\"", value);
            if v.is_sensitive && !v.visible_runtime {
                println!("# ");
            }
            println!(
                "platform variable:create --no-wait --yes --level=environment --project={} --environment={} --name='{}' --value='{}' --json={} --sensitive={} --visible-build={} --visible-runtime={} --enabled={} --inheritable={}",
                args.destination,
                v.environment,
                v.name,
                value,
                v.is_json,
                v.is_sensitive,
                v.visible_build,
                v.visible_runtime,
                v.is_enabled,
                v.is_inheritable
            );
        }
        println!(
            "platform redeploy --project={} --environment={}",
            args.destination, environment
        );
    }

    Ok(())
}

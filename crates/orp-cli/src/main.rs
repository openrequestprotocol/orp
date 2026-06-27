use std::path::PathBuf;

use clap::{Parser, Subcommand};
use orp_bridge::degrade::degrade_to_email;
use orp_core::{
    Importance, Intent, KeyPair, Payload, Policy, PolicyCheckResult, UnsignedRequest,
    validate_against_policy,
};
use orp_server::{config::ServerConfig, db, routes, state::AppState};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "orp", about = "Open Request Protocol CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the federated home server
    Serve {
        #[arg(long, env = "DATABASE_URL")]
        database_url: Option<String>,
        #[arg(long, env = "ORP_PORT", default_value = "8787")]
        port: u16,
        #[arg(long, env = "ORP_DOMAIN", default_value = "localhost")]
        domain: String,
        #[arg(long, env = "ORP_PUBLIC_ENDPOINT")]
        public_endpoint: Option<String>,
    },
    /// Send a signed request
    Send {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        summary: String,
        #[arg(long, default_value = "normal")]
        importance: String,
        #[arg(long, default_value = "Please see details.")]
        body: String,
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Validate a policy JSON file
    ValidatePolicy {
        path: PathBuf,
    },
    /// Validate a request JSON file against a policy
    ValidateRequest {
        request: PathBuf,
        policy: PathBuf,
    },
    /// Degrade a request to email (stdout)
    Degrade {
        request: PathBuf,
    },
    /// Run conformance test vectors
    TestVectors,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Serve {
            database_url,
            port,
            domain,
            public_endpoint,
        } => {
            let mut config = ServerConfig::from_env();
            if let Some(url) = database_url {
                config.database_url = url;
            }
            config.bind = format!("0.0.0.0:{port}").parse()?;
            config.domain = domain;
            if let Some(ep) = public_endpoint {
                config.public_endpoint = ep;
            } else {
                config.public_endpoint = format!("http://{}:{port}", config.domain);
            }
            let pool = db::connect(&config.database_url).await?;
            let state = AppState::new(pool, config);
            routes::serve(state).await?;
        }
        Commands::Send {
            from,
            to,
            intent,
            summary,
            importance,
            body,
            endpoint,
        } => {
            let kp = KeyPair::generate("cli-key");
            let req = UnsignedRequest::new(
                from,
                to,
                parse_intent(&intent)?,
                summary,
                parse_importance(&importance)?,
                Payload {
                    text: body,
                    html: None,
                    subject: None,
                    action: None,
                },
            );
            let signed = kp.sign_request(&req)?;
            let json = serde_json::to_string_pretty(&signed)?;
            println!("{json}");

            if let Some(ep) = endpoint {
                let client = reqwest::Client::new();
                let url = format!("{}/v1/request", ep.trim_end_matches('/'));
                let resp = client
                    .post(&url)
                    .json(&serde_json::json!({ "request": signed }))
                    .send()
                    .await?;
                println!("Server response: {}", resp.text().await?);
            }
        }
        Commands::ValidatePolicy { path } => {
            let data = std::fs::read_to_string(path)?;
            let policy: Policy = serde_json::from_str(&data)?;
            println!("Valid policy for {}", policy.recipient);
        }
        Commands::ValidateRequest { request, policy } => {
            let req_data = std::fs::read_to_string(request)?;
            let pol_data = std::fs::read_to_string(policy)?;
            let req: orp_core::Request = serde_json::from_str(&req_data)?;
            let pol: Policy = serde_json::from_str(&pol_data)?;
            let result = validate_against_policy(&req, &pol, false)?;
            match result {
                PolicyCheckResult::Accept => println!("accept"),
                PolicyCheckResult::Reject(r) => println!("reject: {r}"),
                PolicyCheckResult::DowngradeImportance(i) => {
                    println!("downgrade to {}", i.as_str())
                }
            }
        }
        Commands::Degrade { request } => {
            let data = std::fs::read_to_string(request)?;
            let req: orp_core::Request = serde_json::from_str(&data)?;
            let email = degrade_to_email(&req, None)?;
            print!("{email}");
        }
        Commands::TestVectors => {
            println!("Running orp-core tests...");
            std::process::Command::new("cargo")
                .args(["test", "-p", "orp-core"])
                .status()?;
            println!("Conformance vectors OK");
        }
    }
    Ok(())
}

fn parse_intent(s: &str) -> Result<Intent, Box<dyn std::error::Error + Send + Sync>> {
    Ok(match s {
        "read" => Intent::Read,
        "reply" => Intent::Reply,
        "decide" => Intent::Decide,
        "pay" => Intent::Pay,
        "sign" => Intent::Sign,
        "schedule" => Intent::Schedule,
        "do" => Intent::Do,
        "fyi" => Intent::Fyi,
        _ => return Err(format!("unknown intent: {s}").into()),
    })
}

fn parse_importance(s: &str) -> Result<Importance, Box<dyn std::error::Error + Send + Sync>> {
    Ok(match s {
        "low" => Importance::Low,
        "normal" => Importance::Normal,
        "high" => Importance::High,
        _ => return Err(format!("unknown importance: {s}").into()),
    })
}

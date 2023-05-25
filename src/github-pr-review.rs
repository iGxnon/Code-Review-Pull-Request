use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use github_flows::{listen_to_event, GithubLogin};
use handler::handler;
use std::env;

mod handler;
mod prompt;
mod settings;
mod utils;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() -> anyhow::Result<()> {
    dotenv().ok();
    logger::init();
    log::debug!("Running function at github-pr-review/main");

    let owner = env::var("github_owner").unwrap_or("juntao".to_string());
    let repo = env::var("github_repo").unwrap_or("test".to_string());
    let setting = settings::Settings::from_env();

    let events = vec!["pull_request", "issue_comment"];
    println!("MAGIC");
    listen_to_event(&GithubLogin::Default, &owner, &repo, events, |payload| {
        handler(setting, &owner, &repo, payload)
    })
    .await;

    Ok(())
}

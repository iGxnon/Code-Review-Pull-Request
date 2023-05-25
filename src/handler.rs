use crate::utils::truncate;
use crate::{prompt::format_prompt, settings::Settings};
use github_flows::{
    get_octo,
    octocrab::models::{
        events::payload::{IssueCommentEventAction, PullRequestEventAction},
        CommentId,
    },
    EventPayload, GithubLogin,
};
use http_req::{
    request::{Method, Request},
    uri::Uri,
};
use openai_flows::{chat::ChatOptions, OpenAIFlows};
use std::collections::HashMap;

static BOT_GREETING: &str = "Hello, I am a [code review bot](https://github.com/flows-network/github-pr-review/) on [flows.network](https://flows.network/).\n\n";
static BOT_GREETING_CHECK: &str = "Hello, I am a [code review bot]"; // To check if the comment is from the bot, TODO make it more robust
static BOT_WAITING_PLAYHOLDER: &str =
    "It could take a few minutes for me to analyze this PR. Please be patient.\n";
static BOT_REVIEWS_HEADER: &str =
    "Here are my reviews of changed source code files in this PR.\n\n------\n\n";

pub(crate) async fn handler(setting: Settings, owner: &str, repo: &str, payload: EventPayload) {
    // Whether the PR commit is new or not, commit to an opened PR will trigger PullRequestEventAction::Synchronize event
    // and the bot will summarize this commit again and append it to the old PR comment.
    let mut new_commit: bool = false;
    let (subject, pull_number, _contributor) = match payload {
        EventPayload::PullRequestEvent(e) => {
            if e.action == PullRequestEventAction::Opened {
                log::debug!("Received payload: PR Opened");
            } else if e.action == PullRequestEventAction::Synchronize {
                new_commit = true;
                log::debug!("Received payload: PR Synced");
            } else {
                log::debug!("Not a Opened or Synchronize event for PR");
                return;
            }
            let p = e.pull_request;
            (
                p.title.unwrap_or("".to_string()),
                p.number,
                p.user.unwrap().login,
            )
        }
        EventPayload::IssueCommentEvent(e) => {
            if e.action == IssueCommentEventAction::Deleted {
                log::debug!("Deleted issue comment");
                return;
            }
            log::debug!("Other event for issue comment");

            let body = e.comment.body.unwrap_or_default();

            if body.starts_with(BOT_GREETING_CHECK) {
                log::info!("Ignore comment via bot");
                return;
            };

            if !body
                .to_lowercase()
                .contains(&setting.trigger_phrase.to_lowercase())
            {
                log::info!("Ignore the comment without magic words");
                return;
            }

            (e.issue.title, e.issue.number, e.issue.user.login)
        }
        _ => return,
    };

    let octo = get_octo(&GithubLogin::Default);
    let issues = octo.issues(owner, repo);
    let mut comment_id: CommentId = 0u64.into();
    if new_commit {
        // Find the first BOT_GREETING_CHECK comment to update
        match issues.list_comments(pull_number).send().await {
            Ok(comments) => {
                for c in comments.items {
                    if c.body.unwrap_or_default().starts_with(BOT_GREETING_CHECK) {
                        comment_id = c.id;
                        break;
                    }
                }
            }
            Err(error) => {
                log::error!("Error getting comments: {}", error);
                return;
            }
        }
    } else {
        // PR OPEN or Trigger phrase: create a new comment
        match issues
            .create_comment(
                pull_number,
                format!("{}{}", BOT_GREETING, BOT_WAITING_PLAYHOLDER),
            )
            .await
        {
            Ok(comment) => {
                comment_id = comment.id;
            }
            Err(error) => {
                log::error!("Error posting comment: {}", error);
                return;
            }
        }
    }
    // Return if no comment is found
    if comment_id == 0u64.into() {
        log::error!("Error no comment is found");
        return;
    }

    let chat_id = format!("PR#{pull_number}");
    let system = format_prompt(
        setting.prompt.system_prompt.as_str(),
        HashMap::from([("subject", subject.as_str())]),
    )
    .unwrap_or_default();
    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(3);
    let pulls = octo.pulls(owner, repo);
    let mut resp = String::new();
    resp.push_str(format!("{}{}", BOT_GREETING, BOT_REVIEWS_HEADER).as_str());

    match pulls.list_files(pull_number).await {
        Ok(files) => {
            for f in files.items {
                let filename = &f.filename;
                if !setting.check_file_type(filename) {
                    continue;
                }

                // The f.raw_url is a redirect. So, we need to construct our own here.
                let contents_url = f.contents_url.as_str();
                if contents_url.len() < 40 {
                    continue;
                }
                let hash = &contents_url[(contents_url.len() - 40)..];
                let raw_url = format!(
                    "https://raw.githubusercontent.com/{owner}/{repo}/{}/{}",
                    hash, filename
                );
                let file_uri = Uri::try_from(raw_url.as_str()).unwrap();
                let mut writer = Vec::new();
                if Request::new(&file_uri)
                    .method(Method::GET)
                    .header("Accept", "plain/text")
                    .header("User-Agent", "Flows Network Connector")
                    .send(&mut writer)
                    .map_err(|_e| {})
                    .is_err()
                {
                    log::error!("Cannot get file");
                    continue;
                }
                let file_as_text = String::from_utf8_lossy(&writer);
                let t_file_as_text = truncate(&file_as_text);

                resp.push_str("## [");
                resp.push_str(filename);
                resp.push_str("](");
                resp.push_str(f.blob_url.as_str());
                resp.push_str(")\n\n");

                log::debug!("Sending file to OpenAI: {}", filename);
                let co = ChatOptions {
                    model: setting.model.into(),
                    restart: true,
                    system_prompt: Some(system.as_str()),
                };
                let question = format_prompt(
                    setting.prompt.review_code.as_str(),
                    HashMap::from([("code_message", t_file_as_text)]),
                )
                .unwrap_or_default();
                match openai.chat_completion(&chat_id, &question, &co).await {
                    Ok(r) => {
                        resp.push_str(&r.choice);
                        resp.push_str("\n\n");
                        log::debug!("Received OpenAI resp for file: {}", filename);
                    }
                    Err(e) => {
                        log::error!(
                            "OpenAI returns error for file review for {}: {}",
                            filename,
                            e
                        );
                    }
                }

                log::debug!("Sending patch to OpenAI: {}", filename);
                let co = ChatOptions {
                    model: setting.model.into(),
                    restart: false,
                    system_prompt: Some(system.as_str()),
                };
                let patch_as_text = f.patch.unwrap_or_default();
                let t_patch_as_text = truncate(&patch_as_text);
                let question = format_prompt(
                    setting.prompt.summarize_diff.as_str(),
                    HashMap::from([("patch_message", t_patch_as_text)]),
                )
                .unwrap_or_default();
                match openai.chat_completion(&chat_id, &question, &co).await {
                    Ok(r) => {
                        resp.push_str(&r.choice);
                        resp.push_str("\n\n");
                        log::debug!("Received OpenAI resp for patch: {}", filename);
                    }
                    Err(e) => {
                        log::error!(
                            "OpenAI returns error for patch review for {}: {}",
                            filename,
                            e
                        );
                    }
                }
            }
        }
        Err(_error) => {
            log::error!("Cannot get file list");
        }
    }

    if let Err(err) = issues.update_comment(comment_id, resp).await {
        log::error!("Error posting resp: {}", err);
    }
}

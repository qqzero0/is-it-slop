use jiff::Timestamp;
use serde::Deserialize;
use ureq::Agent;

#[derive(Deserialize, Debug)]
pub struct GithubRepoDetails {
    pub created_at: Timestamp,
}

pub fn fetch_repo_details(
    github_project: &str,
    agent: &Agent,
) -> color_eyre::Result<GithubRepoDetails> {
    agent
        .get(String::from("https://api.github.com/repos/") + github_project)
        .call()?
        .body_mut()
        .read_json()
        .map_err(color_eyre::Report::from)
}

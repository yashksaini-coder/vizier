//! Fetch crate metadata from crates.io API and optional GitHub repo metrics.
//! Uses timeout and response size limit for safety. Intended to be run from a background thread.

use std::time::Duration;

/// Optional GitHub repository metrics (from GitHub REST API).
#[derive(Clone, Debug, Default)]
pub struct GitHubRepoInfo {
    pub stars: Option<u32>,
    pub forks: Option<u32>,
    pub language: Option<String>,
    pub updated_at: Option<String>,
    pub open_issues_count: Option<u32>,
    pub default_branch: Option<String>,
}

/// Crate metadata from crates.io (for inspector docs view). May include GitHub metrics if repo URL is GitHub.
#[derive(Clone, Debug)]
pub struct CrateDocInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub github: Option<GitHubRepoInfo>,
}

/// Max response body size (1 MiB) to avoid unbounded memory.
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;
/// Max GitHub API response (small JSON).
const MAX_GITHUB_RESPONSE_BYTES: u64 = 64 * 1024;
/// Request timeout.
const TIMEOUT: Duration = Duration::from_secs(15);
/// User-Agent: crates.io requires it for API requests.
const USER_AGENT: &str =
    "Vizier/0.1 (Rust code inspector; https://github.com/yashksaini-coder/vizier)";

/// Parse "https://github.com/owner/repo" or "https://github.com/owner/repo/" into Some(("owner", "repo")).
fn parse_github_url(repo: &str) -> Option<(String, String)> {
    let s = repo.trim().trim_end_matches('/');
    let rest = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))?;
    let mut parts = rest.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo_name = parts.next()?.split('/').next()?.to_string();
    if owner.is_empty() || repo_name.is_empty() {
        return None;
    }
    Some((owner, repo_name))
}

/// Fetch repository metrics from GitHub REST API. Returns None on any error.
/// GitHub allows 60 req/h unauthenticated; set GITHUB_TOKEN for 5000/h.
fn fetch_github_repo_info(owner: &str, repo: &str) -> Option<GitHubRepoInfo> {
    let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .ok()?;
    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
    }
    let response = req.send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().ok()?;
    if bytes.len() as u64 > MAX_GITHUB_RESPONSE_BYTES {
        return None;
    }
    let body: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let stars = body
        .get("stargazers_count")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    let forks = body
        .get("forks_count")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    let language = body
        .get("language")
        .and_then(|v| v.as_str())
        .map(String::from);
    let updated_at = body
        .get("updated_at")
        .and_then(|v| v.as_str())
        .map(String::from);
    let open_issues_count = body
        .get("open_issues_count")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    let default_branch = body
        .get("default_branch")
        .and_then(|v| v.as_str())
        .map(String::from);
    Some(GitHubRepoInfo {
        stars,
        forks,
        language,
        updated_at,
        open_issues_count,
        default_branch,
    })
}

/// Fetch crate info from crates.io API. Returns `None` on any error (network, parse, timeout).
/// If the crate has a GitHub repository URL, also fetches repo metrics (stars, forks, language, etc.).
/// Set optional `GITHUB_TOKEN` env var for higher GitHub API rate limit.
/// Safe to call from a background thread; uses blocking HTTP with timeout and size limit.
pub fn fetch_crate_docs(crate_name: &str) -> Option<CrateDocInfo> {
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .ok()?;
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let content_len = response.content_length().unwrap_or(0);
    if content_len > MAX_RESPONSE_BYTES {
        return None;
    }
    let body: serde_json::Value = response.json().ok()?;
    let crate_obj = body.get("crate")?;
    let name = crate_obj.get("name")?.as_str()?.to_string();
    let description = crate_obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);
    let documentation = crate_obj
        .get("documentation")
        .and_then(|v| v.as_str())
        .map(String::from);
    let homepage = crate_obj
        .get("homepage")
        .and_then(|v| v.as_str())
        .map(String::from);
    let repository = crate_obj
        .get("repository")
        .and_then(|v| v.as_str())
        .map(String::from);
    let version = crate_obj
        .get("newest_version")
        .or_else(|| crate_obj.get("max_version"))
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string();

    let github = repository
        .as_ref()
        .and_then(|r| parse_github_url(r))
        .and_then(|(owner, repo)| fetch_github_repo_info(&owner, &repo));

    Some(CrateDocInfo {
        name,
        version,
        description,
        documentation,
        homepage,
        repository,
        github,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url() {
        assert_eq!(
            parse_github_url("https://github.com/rust-lang/rust"),
            Some(("rust-lang".into(), "rust".into()))
        );
        assert_eq!(
            parse_github_url("https://github.com/owner/repo/"),
            Some(("owner".into(), "repo".into()))
        );
        assert_eq!(
            parse_github_url("http://github.com/a/b"),
            Some(("a".into(), "b".into()))
        );
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_none());
        assert!(parse_github_url("https://github.com/").is_none());
        assert!(parse_github_url("").is_none());
    }
}

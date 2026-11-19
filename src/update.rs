//! Update check against the project's GitHub releases.
//!
//! A plain, anonymous API request (no personal data, generic user agent).
//! Runs on a background thread; the result is delivered over a channel.

use std::cmp::Ordering;
use std::process::Command;
use std::sync::mpsc::Receiver;

pub const RELEASES_API: &str = "https://api.github.com/repos/nomadslayer/freepaint/releases?per_page=1";

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub enum UpdateResult {
    UpToDate(String),
    Available(UpdateInfo),
    Failed(String),
}

/// Compare two versions like "0.1.1-alpha": numeric core first, then the
/// pre-release tag (a plain release is newer than any of its pre-releases).
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    fn parse(v: &str) -> (Vec<u64>, Option<String>) {
        let v = v.trim().trim_start_matches('v');
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, Some(p.to_string())),
            None => (v, None),
        };
        let nums = core.split('.').map(|n| n.parse().unwrap_or(0)).collect();
        (nums, pre)
    }
    let (ca, pa) = parse(a);
    let (cb, pb) = parse(b);
    for i in 0..ca.len().max(cb.len()) {
        let x = ca.get(i).copied().unwrap_or(0);
        let y = cb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    match (pa, pb) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => x.cmp(&y),
    }
}

/// First string value of `"key": "..."` in a JSON text.
fn json_string(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let at = json.find(&pat)? + pat.len();
    let rest = &json[at..];
    let start = rest.find('"')? + 1;
    let rest = &rest[start..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => {
                if let Some(e) = chars.next() {
                    out.push(e);
                }
            }
            _ => out.push(c),
        }
    }
    None
}

fn fetch_latest() -> Result<UpdateInfo, String> {
    let out = Command::new("curl")
        .args(["-fsS", "--max-time", "10", "-H", "User-Agent: freepaint-update-check", "-H", "Accept: application/vnd.github+json", RELEASES_API])
        .output()
        .map_err(|e| format!("could not run curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("network request failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let body = String::from_utf8_lossy(&out.stdout);
    let version = json_string(&body, "tag_name").ok_or("no releases found")?;
    let url = json_string(&body, "html_url").unwrap_or_else(|| "https://github.com/nomadslayer/freepaint/releases".into());
    Ok(UpdateInfo { version, url })
}

pub fn check(current: &str) -> UpdateResult {
    match fetch_latest() {
        Ok(info) => {
            if compare_versions(&info.version, current) == Ordering::Greater {
                UpdateResult::Available(info)
            } else {
                UpdateResult::UpToDate(info.version)
            }
        }
        Err(e) => UpdateResult::Failed(e),
    }
}

/// Run the check on a background thread.
pub fn spawn_check(current: String) -> Receiver<UpdateResult> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(check(&current));
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering() {
        use Ordering::*;
        for (a, b, o) in [
            ("v0.1.0-alpha", "0.1.0-alpha", Equal),
            ("0.1.1-alpha", "0.1.0-alpha", Greater),
            ("0.1.0", "0.1.0-alpha", Greater),
            ("0.1.0-alpha", "0.1.0-beta", Less),
            ("0.2.0", "0.1.9", Greater),
            ("1.0.0", "0.9.9-rc", Greater),
            ("0.1.0", "0.1", Equal),
        ] {
            assert_eq!(compare_versions(a, b), o, "{a} vs {b}");
        }
    }

    #[test]
    fn json_extraction() {
        let j = r#"[{"url":"x","html_url":"https://g/h","id":1,"tag_name":"v0.1.0-alpha"}]"#;
        assert_eq!(json_string(j, "tag_name").as_deref(), Some("v0.1.0-alpha"));
        assert_eq!(json_string(j, "html_url").as_deref(), Some("https://g/h"));
    }
}

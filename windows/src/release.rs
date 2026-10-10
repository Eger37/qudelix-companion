//! Platform-independent helpers for GitHub-release self-update (host-testable):
//! semantic-version compare and the few fields we read from the release JSON.

/// "1.4.10" → (1, 4, 10); garbage → (0, 0, 0).
pub fn semver(v: &str) -> (u32, u32, u32) {
    let mut it = v.split('.').map(|p| p.trim().parse::<u32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

/// Value of the first `"key": "value"` string field.
pub fn json_str(json: &str, key: &str) -> Option<String> {
    let k = format!("\"{key}\"");
    let rest = &json[json.find(&k)? + k.len()..];
    let rest = &rest[rest.find('"')? + 1..];
    Some(rest[..rest.find('"')?].to_string())
}

/// `browser_download_url` of the asset whose URL ends with `/<name>`.
pub fn asset_url(json: &str, name: &str) -> Option<String> {
    let key = "\"browser_download_url\"";
    let mut rest = json;
    while let Some(i) = rest.find(key) {
        rest = &rest[i + key.len()..];
        let s = &rest[rest.find('"')? + 1..];
        let url = &s[..s.find('"')?];
        if url.ends_with(&format!("/{name}")) {
            return Some(url.to_string());
        }
    }
    None
}

pub fn split_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://")?;
    let slash = rest.find('/')?;
    Some((rest[..slash].to_string(), rest[slash..].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_json() {
        let j = r#"{"tag_name": "v1.5.2", "assets": [
            {"name":"qudelix-widget.apk","browser_download_url":"https://github.com/o/r/releases/download/v1.5.2/qudelix-widget.apk"},
            {"name":"qudelix-widget.exe","browser_download_url":"https://github.com/o/r/releases/download/v1.5.2/qudelix-widget.exe"}]}"#;
        assert_eq!(json_str(j, "tag_name").as_deref(), Some("v1.5.2"));
        assert_eq!(
            asset_url(j, "qudelix-widget.exe").as_deref(),
            Some("https://github.com/o/r/releases/download/v1.5.2/qudelix-widget.exe")
        );
        assert!(semver("1.5.10") > semver("1.5.9"));
        assert!(semver("1.5.0") > semver("1.4.99"));
        assert_eq!(split_url("https://h.com/a/b"), Some(("h.com".into(), "/a/b".into())));
    }
}

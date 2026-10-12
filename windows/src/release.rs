//! Platform-independent helpers for GitHub-release self-update (host-testable):
//! semantic-version compare, the few fields we read from the release JSON, and
//! the Ed25519 check of a downloaded file.

/// Public half of the release signing key. tools/release.sh signs every
/// release asset with the private half, which lives only on the owner's
/// machine (~/.config/qudelix-companion/release-ed25519.pem), never in git.
/// Print it with: openssl pkey -in <key> -pubout -outform DER | tail -c 32 | xxd -p -c 32
pub const RELEASE_PUBKEY: [u8; 32] = [
    0xa4, 0x9a, 0x7f, 0xb1, 0x61, 0x94, 0xa7, 0x06, 0xff, 0xa1, 0x3c, 0x5c, 0xbf, 0xb6, 0xf4, 0xc9,
    0x11, 0xa0, 0x25, 0x42, 0x05, 0xc8, 0x3a, 0xda, 0xc6, 0xc3, 0x9f, 0x7f, 0x72, 0xef, 0x09, 0x5a,
];

/// True if `sig` (raw 64 bytes, `<asset>.sig`) is a valid signature of `data` by `pubkey`.
pub fn signature_ok(pubkey: &[u8; 32], data: &[u8], sig: &[u8]) -> bool {
    let (Ok(pk), Ok(sig)) = (
        ed25519_compact::PublicKey::from_slice(pubkey),
        ed25519_compact::Signature::from_slice(sig),
    ) else {
        return false;
    };
    pk.verify(data, &sig).is_ok()
}

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

    /// Signature made by `openssl pkeyutl -sign -rawin` (as tools/release.sh
    /// does) with a throwaway test key — not the release key.
    #[test]
    fn verifies_openssl_ed25519_signature() {
        let pk: [u8; 32] = [
            0x4d, 0xdc, 0xaf, 0xaa, 0x61, 0xee, 0x22, 0x21, 0x75, 0x47, 0x27, 0xe7, 0xa4, 0x2b, 0x8a, 0x89,
            0x1e, 0xfe, 0xc7, 0x3e, 0xaa, 0xc3, 0xce, 0xe7, 0xe7, 0x1e, 0xe1, 0xa8, 0xeb, 0x6e, 0xd6, 0xd7,
        ];
        let sig: [u8; 64] = [
            0xea, 0x8c, 0xd9, 0xb4, 0x79, 0xfb, 0x8a, 0x3b, 0x41, 0x4c, 0xac, 0xa3, 0x70, 0x00, 0x96, 0x7b,
            0x90, 0x98, 0xff, 0x41, 0x88, 0x8e, 0x68, 0x15, 0x23, 0x43, 0xa6, 0xe0, 0xc5, 0x9b, 0xf0, 0xfd,
            0x0f, 0xf2, 0xf1, 0x05, 0x95, 0x4b, 0xdb, 0xd7, 0xe4, 0x78, 0x1b, 0x47, 0xd9, 0xa0, 0x1c, 0x55,
            0xc8, 0x2f, 0x03, 0x09, 0xf9, 0x81, 0x1d, 0xfb, 0xad, 0x1b, 0x0d, 0xa8, 0x3b, 0xd6, 0x5e, 0x0f,
        ];
        let msg = b"qudelix-widget test payload";
        assert!(signature_ok(&pk, msg, &sig));
        assert!(!signature_ok(&pk, b"qudelix-widget test payloaD", &sig), "tampered data");
        assert!(!signature_ok(&RELEASE_PUBKEY, msg, &sig), "other key");
        assert!(!signature_ok(&pk, msg, &sig[..63]), "short signature");
        assert!(!signature_ok(&pk, msg, b""), "missing signature");
    }
}

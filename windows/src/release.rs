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

/// Public half of the BACKUP release key, kept offline by the owner. Used only
/// if the main key is lost or leaked: sign one release with it that carries a
/// new key pair, and installed widgets move over by themselves.
pub const BACKUP_PUBKEY: [u8; 32] = [
    0xbc, 0xec, 0x24, 0xa3, 0x51, 0xb2, 0xbe, 0xe7, 0x76, 0x77, 0xd8, 0xf1, 0x22, 0xc2, 0x9b, 0xd1,
    0x16, 0xe7, 0xdb, 0x6e, 0x32, 0x7b, 0xfe, 0x44, 0xe6, 0x7d, 0x5a, 0xb3, 0x02, 0x88, 0x7d, 0xf8,
];

/// Keys a self-update may be signed with.
pub const TRUSTED_KEYS: [&[u8; 32]; 2] = [&RELEASE_PUBKEY, &BACKUP_PUBKEY];

/// True if `sig` is a valid signature of `data` by any of the trusted keys.
pub fn signed_by_trusted(data: &[u8], sig: &[u8]) -> bool {
    TRUSTED_KEYS.iter().any(|k| signature_ok(k, data, sig))
}

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
        assert!(!signed_by_trusted(msg, &sig), "test key is not trusted");
        // The real keys (signatures of the same payload by the main and the backup key).
        let by_main: [u8; 64] = [
            0x9b, 0x9b, 0x75, 0x0d, 0xde, 0xff, 0x7a, 0xc6, 0x41, 0xeb, 0x91, 0x44, 0x99, 0x58, 0x29, 0x47,
            0x66, 0x70, 0x6f, 0xd6, 0x10, 0x29, 0x1b, 0x73, 0xb6, 0xaf, 0x2f, 0x52, 0x78, 0x9b, 0x86, 0x77,
            0xc9, 0xe2, 0xfc, 0xfd, 0xb4, 0xa2, 0xc3, 0xde, 0x57, 0x4d, 0x59, 0x88, 0x0c, 0xb0, 0x34, 0x53,
            0xc6, 0x2c, 0x33, 0xcd, 0xa3, 0xf5, 0x61, 0xb4, 0x55, 0x64, 0xc3, 0xdf, 0x25, 0x1a, 0x65, 0x09,
        ];
        let by_backup: [u8; 64] = [
            0xa3, 0x95, 0x80, 0x39, 0xee, 0x5b, 0xfd, 0xaa, 0x77, 0x18, 0x1e, 0x9f, 0x9d, 0x8f, 0x52, 0x1e,
            0xd7, 0x2e, 0x8a, 0x09, 0x75, 0xd6, 0x0b, 0xe4, 0x53, 0x50, 0xf0, 0x3b, 0x17, 0xea, 0x4b, 0x38,
            0x26, 0x13, 0xe8, 0x63, 0xfa, 0x52, 0x33, 0x13, 0x75, 0x70, 0xa5, 0x81, 0x4b, 0x9b, 0xa0, 0xea,
            0x08, 0xf2, 0xcf, 0x8c, 0xc9, 0xb3, 0x7a, 0x93, 0xb4, 0x68, 0x13, 0x74, 0xa1, 0xe4, 0x01, 0x09,
        ];
        assert!(signed_by_trusted(msg, &by_main), "main release key");
        assert!(signed_by_trusted(msg, &by_backup), "backup release key");
        assert!(!signed_by_trusted(b"other", &by_backup), "backup key, tampered data");
        assert!(!signature_ok(&pk, msg, &sig[..63]), "short signature");
        assert!(!signature_ok(&pk, msg, b""), "missing signature");
    }
}

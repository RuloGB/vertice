//! `app.asar` archive reader — read-only, single-purpose.
//!
//! The ONLY module in `vertice-core` that knows the asar container's byte
//! layout. It extracts exactly one thing: the `version` string of the
//! archive's root `package.json`. It never extracts a file to disk, never
//! enumerates the archive for a caller, and exposes no writer.
//!
//! The header is JSON and is parsed through the `jsonc` seam like every
//! other JSON document in this crate. Every failure is a typed
//! [`AsarError`]; nothing here panics, unwraps an I/O result, or indexes a
//! slice unchecked.
//!
//! See `openspec/changes/detect-desktop-client-installs/design.md` §2 for
//! the byte layout, the six `parse_prefix` cross-checks, and the D1/D2/D3
//! defense-in-depth against a systematic offset error landing on a
//! neighbouring, valid, `name`-bearing `package.json` inside `node_modules`.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::jsonc::{self, JsonValue};

/// The most header bytes this module will read, allocate or parse (§3.3).
pub const HEADER_MAX_BYTES: u32 = 4 * 1024 * 1024;

/// The most bytes this module will read for one archived entry (§2.4).
pub const ENTRY_MAX_BYTES: u64 = 1024 * 1024;

/// Every failure mode this module can produce. `Detected` never depends on
/// which variant fires (design §5.1) — the taxonomy in `installations.rs`
/// grades every variant `Error` except [`AsarError::HeaderTooLarge`], which
/// is `Warning` (design §5.2).
#[derive(Debug, thiserror::Error)]
pub enum AsarError {
    #[error("could not read the archive: {0}")]
    Io(#[from] std::io::Error),
    /// The 16-byte prefix is not a self-consistent asar prefix, or the
    /// header text is not a JSON object. Carries a fixed discriminator, not
    /// a formatted string, so `ScanIssue` reasons stay stable copy.
    #[error("not a readable asar archive: {0}")]
    Malformed(&'static str),
    /// The DECLARED header length exceeds `HEADER_MAX_BYTES`. Distinct from
    /// `Malformed` because it maps to a different severity (§5.2).
    #[error("asar header declares {declared} bytes, above the {limit}-byte ceiling")]
    HeaderTooLarge { declared: u64, limit: u64 },
    #[error("asar header is not valid UTF-8")]
    HeaderNotUtf8,
    #[error("could not parse the asar header: {0}")]
    HeaderParse(#[from] jsonc::JsoncError),
    #[error("the archive has no usable root package.json entry: {0}")]
    Entry(&'static str),
    #[error("the archive's package.json has no \"version\" string")]
    NoVersion,
}

/// Read the `version` of the archive's root `package.json`. Reads at most
/// 16 + `HEADER_MAX_BYTES` + `ENTRY_MAX_BYTES` bytes and never the whole
/// archive. Read-only: opened with `File::open`, never `OpenOptions`.
pub fn read_package_version(archive: &Path) -> Result<String, AsarError> {
    let mut file = std::fs::File::open(archive)?;
    let file_len = file.metadata()?.len();
    read_package_version_from(&mut file, file_len)
}

/// The read sequence (design §2.3), factored over any `Read + Seek` so the
/// pure/in-memory test layer (§8.1) can exercise it against a `Cursor` with
/// no file on disk. `read_package_version` is the only caller that ever
/// supplies a real file handle.
fn read_package_version_from<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
) -> Result<String, AsarError> {
    let mut prefix_bytes = [0u8; 16];
    reader.read_exact(&mut prefix_bytes)?;

    let prefix = parse_prefix(&prefix_bytes, file_len)?;

    let header_len = usize::try_from(prefix.json_len)
        .map_err(|_| AsarError::Malformed("json_len does not fit this platform's usize"))?;
    let mut header_bytes = vec![0u8; header_len];
    reader.read_exact(&mut header_bytes)?;

    let header_text = String::from_utf8(header_bytes).map_err(|_| AsarError::HeaderNotUtf8)?;
    let header = jsonc::parse(&header_text)?;
    if !matches!(header, JsonValue::Object(_)) {
        return Err(AsarError::Malformed("header is not a JSON object"));
    }

    let payload_len = file_len
        .checked_sub(prefix.data_start)
        .ok_or(AsarError::Malformed("data_start is beyond the file's end"))?;
    let entry = locate_package_json(&header, payload_len)?;

    let absolute_offset = prefix
        .data_start
        .checked_add(entry.offset)
        .ok_or(AsarError::Entry("root package.json offset overflows"))?;
    reader.seek(SeekFrom::Start(absolute_offset))?;

    let entry_len = usize::try_from(entry.size)
        .map_err(|_| AsarError::Entry("size does not fit this platform's usize"))?;
    let mut entry_bytes = vec![0u8; entry_len];
    reader.read_exact(&mut entry_bytes)?;

    let entry_text = String::from_utf8(entry_bytes)
        .map_err(|_| AsarError::Entry("package.json bytes are not valid UTF-8"))?;
    let package_json = jsonc::parse(&entry_text)?;

    extract_version(&package_json)
}

/// The three fields `read_package_version_from` needs out of the 16-byte
/// prefix, once it has been validated (design §2.2).
#[derive(Debug)]
struct Prefix {
    json_len: u32,
    data_start: u64,
}

/// Validate the 16-byte asar prefix and derive `data_start`. Every step is
/// the `checked_*` call design §2.2 mandates; there is no bare subtraction
/// and no ordering in which arithmetic runs before the guard that protects
/// it. `data_start` is ALWAYS `8 + header_len`, NEVER `json_start +
/// json_len` — see `data_start_is_eight_plus_header_len_not_json_start_plus_json_len`.
fn parse_prefix(prefix: &[u8; 16], file_len: u64) -> Result<Prefix, AsarError> {
    let pickle_header_size = read_u32_le(prefix, 0);
    let header_len = read_u32_le(prefix, 4);
    let header_payload_len = read_u32_le(prefix, 8);
    let json_len = read_u32_le(prefix, 12);

    if pickle_header_size != 4 {
        return Err(AsarError::Malformed(
            "the prefix's pickle header size is not 4",
        ));
    }

    // Check 2: header_payload_len == header_len - 4. The checked_sub IS the
    // header_len >= 4 guard (tiny-header-len fixture, §8.2).
    let expected_payload_len = u64::from(header_len)
        .checked_sub(4)
        .ok_or(AsarError::Malformed("header_len is below 4"))?;
    if u64::from(header_payload_len) != expected_payload_len {
        return Err(AsarError::Malformed(
            "header_payload_len does not equal header_len minus 4",
        ));
    }

    // Check 3: 2 <= json_len <= header_payload_len - 4.
    let max_json_len = u64::from(header_payload_len)
        .checked_sub(4)
        .ok_or(AsarError::Malformed("header_payload_len is below 4"))?;
    if json_len < 2 || u64::from(json_len) > max_json_len {
        return Err(AsarError::Malformed("json_len is out of bounds"));
    }

    // Check 4: the padding to the 4-byte boundary is strictly less than 4.
    let padding = max_json_len
        .checked_sub(u64::from(json_len))
        .ok_or(AsarError::Malformed("json_len exceeds header_payload_len"))?;
    if padding > 3 {
        return Err(AsarError::Malformed(
            "padding is not less than the 4-byte alignment",
        ));
    }

    // Check 5: the ceiling fires BEFORE any allocation and before check 6's
    // data_start-vs-file_len comparison — a fixture whose file is exactly 16
    // bytes long must still reach this branch (oversized-header, §8.2).
    if u64::from(json_len) > u64::from(HEADER_MAX_BYTES) {
        return Err(AsarError::HeaderTooLarge {
            declared: u64::from(json_len),
            limit: u64::from(HEADER_MAX_BYTES),
        });
    }

    // Check 6: data_start = 8 + header_len, and it must fit inside the file.
    let data_start = 8u64
        .checked_add(u64::from(header_len))
        .ok_or(AsarError::Malformed("data_start overflows"))?;
    if data_start > file_len {
        return Err(AsarError::Malformed(
            "data_start is beyond the end of the file",
        ));
    }

    Ok(Prefix {
        json_len,
        data_start,
    })
}

/// Read a little-endian `u32` out of a fixed 16-byte prefix at one of the
/// four constant word offsets (0, 4, 8, 12). No `unwrap`/`expect`/`as`
/// cast: `copy_from_slice` panics only on a length mismatch, which is
/// unreachable here because both sides are always exactly 4 bytes by
/// construction (`offset` is one of the four call sites above, never
/// derived from untrusted input).
fn read_u32_le(prefix: &[u8; 16], offset: usize) -> u32 {
    let mut word = [0u8; 4];
    word.copy_from_slice(&prefix[offset..offset + 4]);
    u32::from_le_bytes(word)
}

/// One resolved, validated `package.json` entry: a byte range relative to
/// `data_start`.
#[derive(Debug)]
struct Entry {
    offset: u64,
    size: u64,
}

/// Look up `root["files"]["package.json"]` and NOWHERE else (A2, design
/// §2.4). No subtree walk, no fallback candidate — the rejected recursive
/// search is exactly what would let a `node_modules` dependency's version
/// be silently reported as OpenCode's.
fn locate_package_json(header: &JsonValue, payload_len: u64) -> Result<Entry, AsarError> {
    let JsonValue::Object(root) = header else {
        return Err(AsarError::Malformed("header is not a JSON object"));
    };

    let files = match root.get("files") {
        Some(JsonValue::Object(files)) => files,
        _ => return Err(AsarError::Entry("no root package.json entry")),
    };

    let entry = match files.get("package.json") {
        Some(JsonValue::Object(entry)) => entry,
        _ => return Err(AsarError::Entry("no root package.json entry")),
    };

    if matches!(entry.get("unpacked"), Some(JsonValue::Bool(true))) {
        return Err(AsarError::Entry("root package.json is unpacked"));
    }
    if entry.contains_key("files") {
        // A "files" key means this is a directory node, not a file entry —
        // asar never shapes a real file entry this way.
        return Err(AsarError::Entry("no root package.json entry"));
    }

    let offset = match entry.get("offset") {
        Some(JsonValue::String(s)) => s
            .parse::<u64>()
            .map_err(|_| AsarError::Entry("offset is not a valid integer"))?,
        Some(JsonValue::Number(s)) => s
            .parse::<u64>()
            .map_err(|_| AsarError::Entry("offset is not a valid integer"))?,
        _ => return Err(AsarError::Entry("offset is missing or the wrong type")),
    };

    let size = match entry.get("size") {
        Some(JsonValue::Number(s)) => s
            .parse::<u64>()
            .map_err(|_| AsarError::Entry("size is not a valid integer"))?,
        _ => return Err(AsarError::Entry("size is missing or the wrong type")),
    };

    if !(2..=ENTRY_MAX_BYTES).contains(&size) {
        return Err(AsarError::Entry("size is out of bounds"));
    }

    // D2: the resolved offset+size must land strictly inside the payload
    // region — the defense against a systematic offset error (§2.4).
    let end = offset
        .checked_add(size)
        .ok_or(AsarError::Entry("offset + size overflows"))?;
    if end > payload_len {
        return Err(AsarError::Entry(
            "root package.json offset is out of the payload",
        ));
    }

    Ok(Entry { offset, size })
}

/// D3: the parsed entry must carry BOTH a non-empty `name` and a non-empty
/// `version`. Deliberately does not assert `name == "opencode"` (design
/// §2.4 — a hardcoded equality would turn a harmless upstream rename into a
/// dead probe); requiring `name` to be present and non-empty is the
/// strongest check available that cannot rot.
fn extract_version(package_json: &JsonValue) -> Result<String, AsarError> {
    let JsonValue::Object(map) = package_json else {
        return Err(AsarError::Entry("package.json is not a JSON object"));
    };

    let has_name = matches!(map.get("name"), Some(JsonValue::String(s)) if !s.is_empty());
    if !has_name {
        return Err(AsarError::Entry(
            "package.json is not shaped like a manifest",
        ));
    }

    match map.get("version") {
        Some(JsonValue::String(s)) if !s.is_empty() => Ok(s.clone()),
        _ => Err(AsarError::NoVersion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Assemble a syntactically valid asar from a header JSON string and a
    /// payload. Deliberately the ONLY place in the test suite that writes
    /// the 16-byte prefix, so the layout appears exactly twice in the
    /// repository: here and in `parse_prefix` (design §8.1).
    fn build_asar(header_json: &str, payload: &[u8]) -> Vec<u8> {
        let header_bytes = header_json.as_bytes();
        let json_len = u32::try_from(header_bytes.len()).expect("test header fits in u32");

        // header_payload_len = round_up_to_4(4 [the json_len field itself]
        // + json_len). Derived and cross-checked against M1's measured real
        // archive in design.md §8.2.
        let unpadded = 4usize + header_bytes.len();
        let padded = unpadded.div_ceil(4) * 4;
        let padding_len = padded - unpadded;
        let header_payload_len = u32::try_from(padded).expect("test header fits in u32");
        let header_len = header_payload_len + 4;

        let mut out = Vec::new();
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&header_len.to_le_bytes());
        out.extend_from_slice(&header_payload_len.to_le_bytes());
        out.extend_from_slice(&json_len.to_le_bytes());
        out.extend_from_slice(header_bytes);
        out.extend(std::iter::repeat_n(0u8, padding_len));
        out.extend_from_slice(payload);
        out
    }

    fn read_version_in_memory(bytes: &[u8]) -> Result<String, AsarError> {
        let file_len = bytes.len() as u64;
        let mut cursor = Cursor::new(bytes.to_vec());
        read_package_version_from(&mut cursor, file_len)
    }

    // -- build_asar / happy path cross-check against design §8.2's 105-byte fixture --

    #[test]
    fn build_asar_reproduces_the_105_byte_happy_fixture() {
        let header = r#"{"files":{"package.json":{"size":37,"offset":"0"}}}"#;
        let payload = br#"{"name":"opencode","version":"0.4.2"}"#;

        let bytes = build_asar(header, payload);

        assert_eq!(bytes.len(), 105, "the happy fixture is exactly 105 bytes");
        assert_eq!(&bytes[0..4], &4u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &60u32.to_le_bytes(), "header_len");
        assert_eq!(&bytes[8..12], &56u32.to_le_bytes(), "header_payload_len");
        assert_eq!(&bytes[12..16], &51u32.to_le_bytes(), "json_len");
        assert_eq!(&bytes[16..67], header.as_bytes());
        assert_eq!(&bytes[67..68], &[0u8], "1 padding byte");
        assert_eq!(&bytes[68..105], payload);
    }

    // -- parse_prefix (§2.2 checks 1-6) --

    #[test]
    fn parse_prefix_rejects_a_prefix_whose_payload_length_disagrees_with_the_header_length() {
        let mut prefix = [0u8; 16];
        prefix[0..4].copy_from_slice(&4u32.to_le_bytes());
        prefix[4..8].copy_from_slice(&60u32.to_le_bytes()); // header_len
        prefix[8..12].copy_from_slice(&999u32.to_le_bytes()); // WRONG payload_len
        prefix[12..16].copy_from_slice(&51u32.to_le_bytes());

        let result = parse_prefix(&prefix, 1000);

        assert!(matches!(result, Err(AsarError::Malformed(_))));
    }

    #[test]
    fn parse_prefix_rejects_a_header_len_below_four_without_underflowing() {
        for header_len in [0u32, 1, 2, 3] {
            let mut prefix = [0u8; 16];
            prefix[0..4].copy_from_slice(&4u32.to_le_bytes());
            prefix[4..8].copy_from_slice(&header_len.to_le_bytes());
            prefix[8..12].copy_from_slice(&0u32.to_le_bytes());
            prefix[12..16].copy_from_slice(&0u32.to_le_bytes());

            let result = parse_prefix(&prefix, 1000);

            assert!(
                matches!(result, Err(AsarError::Malformed(_))),
                "header_len={header_len} must be rejected, not underflow or panic"
            );
        }
    }

    #[test]
    fn parse_prefix_refuses_a_header_above_the_ceiling_without_reading_it() {
        let declared_json_len = 5_000_000u32;
        let header_payload_len = declared_json_len + 4;
        let header_len = header_payload_len + 4;

        let mut prefix = [0u8; 16];
        prefix[0..4].copy_from_slice(&4u32.to_le_bytes());
        prefix[4..8].copy_from_slice(&header_len.to_le_bytes());
        prefix[8..12].copy_from_slice(&header_payload_len.to_le_bytes());
        prefix[12..16].copy_from_slice(&declared_json_len.to_le_bytes());

        // The file is ONLY 16 bytes long, proving the ceiling fires before
        // any read or allocation beyond the prefix itself.
        let result = parse_prefix(&prefix, 16);

        match result {
            Err(AsarError::HeaderTooLarge { declared, limit }) => {
                assert_eq!(declared, u64::from(declared_json_len));
                assert_eq!(limit, u64::from(HEADER_MAX_BYTES));
            }
            other => panic!("expected HeaderTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn data_start_is_eight_plus_header_len_not_json_start_plus_json_len() {
        let header = r#"{"files":{"package.json":{"size":37,"offset":"0"}}}"#;
        let payload = br#"{"name":"opencode","version":"0.4.2"}"#;
        let bytes = build_asar(header, payload);

        let mut prefix_bytes = [0u8; 16];
        prefix_bytes.copy_from_slice(&bytes[0..16]);
        let prefix = parse_prefix(&prefix_bytes, bytes.len() as u64).expect("valid prefix");

        let json_start = 16u64;
        let json_len = u64::from(prefix.json_len);
        let forbidden = json_start + json_len;

        assert_eq!(prefix.data_start, 68, "8 + header_len (8 + 60)");
        assert_ne!(
            prefix.data_start, forbidden,
            "data_start must differ from json_start + json_len on a padded fixture"
        );
    }

    // -- D1/D2/D3 defense in depth (§2.4) --

    #[test]
    fn a_shifted_payload_never_yields_the_neighbouring_manifests_version() {
        // The true root manifest sits AT the declared offset (0), exactly
        // where `data_start` (correctly computed as `8 + header_len`)
        // places it. A second, complete, name-bearing manifest — the kind
        // that densely populates a real archive's `node_modules` payload —
        // sits immediately after it in the SAME payload. The header's
        // declared `size` bounds the read to exactly the root manifest's
        // bytes, so the neighbour is present in the archive but is never
        // touched by `read_package_version`. This is the concrete guard
        // that a systematic offset error (§2.4, the forbidden
        // `json_start + json_len` formula pinned separately by
        // `data_start_is_eight_plus_header_len_not_json_start_plus_json_len`)
        // would land on a plausible, wrong, `name`-bearing manifest instead
        // of failing loudly.
        let root_manifest = br#"{"name":"opencode","version":"0.4.2"}"#;
        let neighbour = br#"{"name":"left-pad","version":"9.9.9"}"#;

        let header = format!(
            r#"{{"files":{{"package.json":{{"size":{},"offset":"0"}}}}}}"#,
            root_manifest.len()
        );

        let mut payload = Vec::new();
        payload.extend_from_slice(root_manifest);
        payload.extend_from_slice(neighbour);

        let bytes = build_asar(&header, &payload);

        assert!(
            String::from_utf8_lossy(&bytes).contains("9.9.9"),
            "sanity: the neighbour bytes are actually present in the archive"
        );

        let version = read_version_in_memory(&bytes).expect("root manifest must be read");
        assert_eq!(version, "0.4.2");
        assert_ne!(version, "9.9.9");
    }

    #[test]
    fn a_version_without_a_name_is_refused() {
        let bare = jsonc::parse(r#"{"version":"0.4.2"}"#).expect("valid json");

        let result = extract_version(&bare);

        match result {
            Err(AsarError::Entry(reason)) => {
                assert!(reason.contains("not shaped like a manifest"));
            }
            other => panic!("expected Entry(..), got {other:?}"),
        }
    }

    #[test]
    fn locate_package_json_ignores_a_nested_node_modules_package_json() {
        let header = jsonc::parse(
            r#"{"files":{"node_modules":{"files":{"package.json":{"size":10,"offset":"0"}}}}}"#,
        )
        .expect("valid json");

        let result = locate_package_json(&header, 1000);

        assert!(matches!(result, Err(AsarError::Entry(_))));
    }

    #[test]
    fn offset_out_of_the_payload_is_rejected() {
        let header = jsonc::parse(r#"{"files":{"package.json":{"size":10,"offset":"9999"}}}"#)
            .expect("valid json");

        let result = locate_package_json(&header, 50);

        match result {
            Err(AsarError::Entry(reason)) => {
                assert!(reason.contains("out of the payload"));
            }
            other => panic!("expected Entry(..), got {other:?}"),
        }
    }

    // -- full in-memory end-to-end --

    #[test]
    fn read_package_version_returns_the_root_version_from_a_synthetic_archive() {
        let header = r#"{"files":{"package.json":{"size":37,"offset":"0"}}}"#;
        let payload = br#"{"name":"opencode","version":"0.4.2"}"#;
        let bytes = build_asar(header, payload);

        let version = read_version_in_memory(&bytes).expect("must read the happy path");

        assert_eq!(version, "0.4.2");
    }

    #[test]
    fn no_root_package_json_entry_is_an_entry_error() {
        let header = r#"{"files":{"README.md":{"size":2,"offset":"0"}}}"#;
        let bytes = build_asar(header, b"ok");

        let result = read_version_in_memory(&bytes);

        assert!(matches!(result, Err(AsarError::Entry(_))));
    }

    #[test]
    fn no_version_key_is_no_version_error() {
        let payload = br#"{"name":"opencode"}"#;
        let header = format!(
            r#"{{"files":{{"package.json":{{"size":{},"offset":"0"}}}}}}"#,
            payload.len()
        );
        let bytes = build_asar(&header, payload);

        let result = read_version_in_memory(&bytes);

        assert!(matches!(result, Err(AsarError::NoVersion)));
    }

    #[test]
    fn empty_version_is_no_version_error() {
        let payload = br#"{"name":"opencode","version":""}"#;
        let header = format!(
            r#"{{"files":{{"package.json":{{"size":{},"offset":"0"}}}}}}"#,
            payload.len()
        );
        let bytes = build_asar(&header, payload);

        let result = read_version_in_memory(&bytes);

        assert!(matches!(result, Err(AsarError::NoVersion)));
    }

    #[test]
    fn malformed_header_json_is_malformed_or_parse_error() {
        let mut prefix = [0u8; 16];
        // A syntactically-consistent but non-JSON header: 8 bytes of junk.
        let header_bytes = b"not json";
        let json_len = header_bytes.len() as u32;
        let unpadded = 4 + header_bytes.len();
        let padded = unpadded.div_ceil(4) * 4;
        let padding_len = padded - unpadded;
        let header_payload_len = padded as u32;
        let header_len = header_payload_len + 4;

        prefix[0..4].copy_from_slice(&4u32.to_le_bytes());
        prefix[4..8].copy_from_slice(&header_len.to_le_bytes());
        prefix[8..12].copy_from_slice(&header_payload_len.to_le_bytes());
        prefix[12..16].copy_from_slice(&json_len.to_le_bytes());

        let mut bytes = prefix.to_vec();
        bytes.extend_from_slice(header_bytes);
        bytes.extend(std::iter::repeat_n(0u8, padding_len));

        let result = read_version_in_memory(&bytes);

        assert!(result.is_err(), "must not panic on malformed header JSON");
    }

    #[test]
    fn unpacked_root_entry_is_refused() {
        let header = r#"{"files":{"package.json":{"size":10,"offset":"0","unpacked":true}}}"#;
        let bytes = build_asar(header, b"0123456789");

        let result = read_version_in_memory(&bytes);

        match result {
            Err(AsarError::Entry(reason)) => assert!(reason.contains("unpacked")),
            other => panic!("expected Entry(..), got {other:?}"),
        }
    }
}

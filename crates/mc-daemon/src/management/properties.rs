use mc_core::{
    AppError, ErrorCode, InstanceSummary, ServerDifficulty, ServerPropertiesSnapshot,
    ServerPropertiesValues,
};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tempfile::NamedTempFile;

use super::{invalid, io_error, safe_path};

const MAX_PROPERTIES_BYTES: u64 = 256 * 1024;
const PROPERTY_KEYS: [&str; 4] = ["motd", "max-players", "difficulty", "view-distance"];
static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn read(instance: &InstanceSummary) -> Result<ServerPropertiesSnapshot, AppError> {
    let path = properties_path(instance)?;
    let bytes = read_bytes(&path)?;
    let raw = String::from_utf8(bytes.clone()).map_err(|error| {
        invalid("server.properties 不是有效 UTF-8，未打开编辑器。").with_evidence(error.to_string())
    })?;
    let (values, warnings, _) = parse_document(&raw);
    Ok(ServerPropertiesSnapshot {
        instance_id: instance.id,
        revision: revision(&bytes),
        values,
        warnings,
    })
}

pub(crate) fn save(
    instance: &InstanceSummary,
    expected_revision: &str,
    values: ServerPropertiesValues,
) -> Result<ServerPropertiesSnapshot, AppError> {
    values.validate()?;
    let lock = WRITE_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = properties_path(instance)?;
    let current_bytes = read_bytes(&path)?;
    if revision(&current_bytes) != expected_revision {
        return Err(concurrent_edit());
    }
    let current = String::from_utf8(current_bytes.clone()).map_err(|error| {
        invalid("server.properties 不是有效 UTF-8，未保存。").with_evidence(error.to_string())
    })?;
    let (_, warnings, _) = parse_document(&current);
    if warnings.iter().any(|warning| warning.contains("多行转义")) {
        return Err(invalid(
            "有配置项使用了多行转义；为避免改变其含义，请先在文件中整理为单行再保存。",
        ));
    }
    let output = rewrite_document(&current, &values);

    // Check once more immediately before the atomic replace. This catches an
    // editor or server rewrite that happened while the new file was prepared.
    if revision(&read_bytes(&path)?) != expected_revision {
        return Err(concurrent_edit());
    }
    atomic_replace(&path, output.as_bytes())?;
    let bytes = output.into_bytes();
    let (saved_values, warnings, _) =
        parse_document(std::str::from_utf8(&bytes).expect("output was built from UTF-8 text"));
    Ok(ServerPropertiesSnapshot {
        instance_id: instance.id,
        revision: revision(&bytes),
        values: saved_values,
        warnings,
    })
}

fn properties_path(instance: &InstanceSummary) -> Result<PathBuf, AppError> {
    safe_path(&instance.directory, Path::new("server/server.properties"))
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    if !file.metadata().map_err(io_error)?.is_file() {
        return Err(invalid("server.properties 必须是普通文件。"));
    }
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(MAX_PROPERTIES_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > MAX_PROPERTIES_BYTES {
        return Err(invalid("server.properties 超过 256 KiB，未载入。"));
    }
    Ok(bytes)
}

fn revision(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug)]
struct PropertyLine {
    body: String,
    ending: &'static str,
}

fn split_lines(text: &str) -> Vec<PropertyLine> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split_inclusive('\n')
        .map(|line| {
            if let Some(line) = line.strip_suffix('\n') {
                if let Some(body) = line.strip_suffix('\r') {
                    PropertyLine {
                        body: body.to_owned(),
                        ending: "\r\n",
                    }
                } else {
                    PropertyLine {
                        body: line.to_owned(),
                        ending: "\n",
                    }
                }
            } else {
                PropertyLine {
                    body: line.to_owned(),
                    ending: "",
                }
            }
        })
        .collect()
}

/// Returns a decoded key and the byte offset where its value begins.
fn line_key_and_value_start(line: &str) -> Option<(String, usize)> {
    let bytes = line.as_bytes();
    let mut cursor = 0;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if cursor == bytes.len() || matches!(bytes[cursor], b'#' | b'!') {
        return None;
    }
    let key_start = cursor;
    let mut escaped = false;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'=' || byte == b':' || byte.is_ascii_whitespace() {
            break;
        }
        cursor += 1;
    }
    let key_raw = &line[key_start..cursor];
    if key_raw.is_empty() {
        return None;
    }
    let key = decode_property_value(key_raw).ok()?;
    let had_whitespace = bytes.get(cursor).is_some_and(u8::is_ascii_whitespace);
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if bytes
        .get(cursor)
        .is_some_and(|byte| matches!(byte, b'=' | b':'))
    {
        cursor += 1;
    } else if !had_whitespace {
        // A key without a separator has an empty value.
        return Some((key, line.len()));
    }
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    Some((key, cursor))
}

fn parse_document(text: &str) -> (ServerPropertiesValues, Vec<String>, Vec<Option<usize>>) {
    let mut values = ServerPropertiesValues::default();
    let mut warnings = Vec::new();
    let lines = split_lines(text);
    let mut last_lines = vec![None; PROPERTY_KEYS.len()];
    for (line_index, line) in lines.iter().enumerate() {
        let Some((key, value_start)) = line_key_and_value_start(&line.body) else {
            continue;
        };
        let Some(property_index) = PROPERTY_KEYS.iter().position(|name| *name == key) else {
            continue;
        };
        last_lines[property_index] = Some(line_index);
        let raw_value = &line.body[value_start..];
        if has_line_continuation(raw_value) {
            warnings.push(format!(
                "{} 使用了多行转义；编辑器暂以安全默认值显示，保存时会收敛为单行。",
                PROPERTY_KEYS[property_index]
            ));
            set_default_value(&mut values, property_index);
            continue;
        }
        let decoded = match decode_property_value(raw_value) {
            Ok(value) => value,
            Err(()) => {
                warnings.push(format!(
                    "{} 包含无法解析的转义；编辑器暂以安全默认值显示。",
                    PROPERTY_KEYS[property_index]
                ));
                set_default_value(&mut values, property_index);
                continue;
            }
        };
        let parsed = match property_index {
            0 => {
                values.motd = decoded.clone();
                true
            }
            1 => decoded
                .parse::<u16>()
                .ok()
                .filter(|value| (1..=1000).contains(value))
                .map(|value| values.max_players = value)
                .is_some(),
            2 => ServerDifficulty::parse(&decoded)
                .map(|value| values.difficulty = value)
                .is_some(),
            3 => decoded
                .parse::<u8>()
                .ok()
                .filter(|value| (3..=32).contains(value))
                .map(|value| values.view_distance = value)
                .is_some(),
            _ => true,
        };
        if !parsed {
            warnings.push(format!(
                "{} 的当前值“{}”不在编辑器支持范围内，暂以安全默认值显示。",
                PROPERTY_KEYS[property_index], decoded
            ));
            set_default_value(&mut values, property_index);
        }
    }
    (values, warnings, last_lines)
}

fn set_default_value(values: &mut ServerPropertiesValues, index: usize) {
    let defaults = ServerPropertiesValues::default();
    match index {
        0 => values.motd = defaults.motd,
        1 => values.max_players = defaults.max_players,
        2 => values.difficulty = defaults.difficulty,
        3 => values.view_distance = defaults.view_distance,
        _ => {}
    }
}

fn rewrite_document(text: &str, values: &ServerPropertiesValues) -> String {
    let mut lines = split_lines(text);
    let (_, _, last_lines) = parse_document(text);
    let replacements = property_values(values);
    for (index, line_index) in last_lines.iter().enumerate() {
        if let Some(line_index) = line_index {
            let line = &mut lines[*line_index];
            let value_start = line_key_and_value_start(&line.body)
                .map(|(_, value_start)| value_start)
                .unwrap_or(line.body.len());
            line.body.truncate(value_start);
            line.body
                .push_str(&escape_property_value(&replacements[index]));
        }
    }
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let present = last_lines.iter().map(Option::is_some).collect::<Vec<_>>();
    for (index, exists) in present.iter().enumerate() {
        if !exists {
            if lines.last().is_some_and(|line| line.ending.is_empty()) {
                lines.last_mut().expect("last line exists").ending = newline;
            }
            lines.push(PropertyLine {
                body: format!(
                    "{}={}",
                    PROPERTY_KEYS[index],
                    escape_property_value(&replacements[index])
                ),
                ending: newline,
            });
        }
    }
    let mut output = String::with_capacity(text.len() + 128);
    for line in lines {
        output.push_str(&line.body);
        output.push_str(line.ending);
    }
    output
}

fn property_values(values: &ServerPropertiesValues) -> [String; 4] {
    [
        values.motd.clone(),
        values.max_players.to_string(),
        values.difficulty.as_str().to_owned(),
        values.view_distance.to_string(),
    ]
}

fn escape_property_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for (index, character) in value.chars().enumerate() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\u{000c}' => escaped.push_str("\\f"),
            ' ' if index == 0 => escaped.push_str("\\ "),
            character if !character.is_ascii() => {
                let mut encoded = [0; 2];
                for unit in character.encode_utf16(&mut encoded) {
                    escaped.push_str(&format!("\\u{unit:04x}"));
                }
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn decode_property_value(value: &str) -> Result<String, ()> {
    let mut utf16 = Vec::new();
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            let mut encoded = [0; 2];
            utf16.extend_from_slice(character.encode_utf16(&mut encoded));
            continue;
        }
        let escaped = chars.next().ok_or(())?;
        match escaped {
            't' => utf16.push('\t' as u16),
            'r' => utf16.push('\r' as u16),
            'n' => utf16.push('\n' as u16),
            'f' => utf16.push('\u{000c}' as u16),
            'u' => {
                let mut code = 0u16;
                for _ in 0..4 {
                    code = code.checked_mul(16).ok_or(())?;
                    code += chars
                        .next()
                        .and_then(|digit| digit.to_digit(16))
                        .ok_or(())? as u16;
                }
                utf16.push(code);
            }
            other => {
                let mut encoded = [0; 2];
                utf16.extend_from_slice(other.encode_utf16(&mut encoded));
            }
        }
    }
    String::from_utf16(&utf16).map_err(|_| ())
}

fn has_line_continuation(value: &str) -> bool {
    value
        .chars()
        .rev()
        .take_while(|character| *character == '\\')
        .count()
        % 2
        == 1
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("无法定位 server.properties 所在目录。"))?;
    std::fs::create_dir_all(parent).map_err(io_error)?;
    let old_permissions = std::fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    let mut temporary = NamedTempFile::new_in(parent).map_err(io_error)?;
    temporary.write_all(bytes).map_err(io_error)?;
    if let Some(permissions) = old_permissions {
        temporary
            .as_file()
            .set_permissions(permissions)
            .map_err(io_error)?;
    }
    temporary.as_file().sync_all().map_err(io_error)?;
    temporary
        .persist(path)
        .map_err(|error| io_error(error.error))?;
    // The rename is the commit point. A directory sync failure after it must
    // not be reported as a failed save because the new file is already live.
    if let Ok(directory) = std::fs::File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn concurrent_edit() -> AppError {
    AppError::new(
        ErrorCode::ConcurrentModification,
        "server.properties 在编辑期间发生了变化；尚未写入。",
    )
    .with_action("按 F5 重新读取后再修改，或先检查文件中的外部变更。")
    .retryable()
}

use camino::Utf8Path;
use loom_types::api::{BoundaryEntry, CassetteRibbon};
use serde_json::Value;

pub fn ribbon(root: &Utf8Path) -> CassetteRibbon {
    let mut boundaries = Vec::new();
    visit_files(root.join(".x07_rr").as_std_path(), &mut |path| {
        let relative = path
            .strip_prefix(root.as_std_path())
            .ok()
            .and_then(|path| path.to_str())
            .unwrap_or_default()
            .replace('\\', "/");
        let raw = std::fs::read_to_string(path).unwrap_or_default();
        let json = serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null);
        let metadata = std::fs::metadata(path).ok();
        let at = string_field(&json, &["at", "ts", "timestamp"])
            .or_else(|| {
                metadata
                    .as_ref()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|modified| {
                        modified
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|duration| duration.as_millis().to_string())
                    })
            })
            .unwrap_or_else(|| "0".to_string());
        let kind = string_field(&json, &["kind", "world", "operation"]).unwrap_or_else(|| {
            path.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("rr")
                .to_string()
        });
        let policy = string_field(&json, &["policy", "policy_id", "rr_policy"])
            .unwrap_or_else(|| "default".to_string());
        let summary = string_field(&json, &["summary", "message"]).unwrap_or_else(|| {
            format!(
                "{} boundary · {} bytes",
                kind,
                metadata.as_ref().map(|item| item.len()).unwrap_or(0)
            )
        });
        boundaries.push(BoundaryEntry {
            at,
            kind,
            policy,
            summary,
            cassette_path: relative,
        });
    });
    boundaries.sort_by(|a, b| a.at.cmp(&b.at).then(a.cassette_path.cmp(&b.cassette_path)));
    CassetteRibbon {
        schema_version: "x07.studio.cassette_ribbon@0.1.0".to_string(),
        boundaries,
    }
}

fn visit_files(dir: &std::path::Path, f: &mut impl FnMut(&std::path::Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, f);
        } else {
            f(&path);
        }
    }
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    find_key(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn find_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| find_key(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| find_key(value, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::ribbon;
    use uuid::Uuid;

    #[test]
    fn ribbon_orders_cassette_boundaries() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-cassette-ribbon-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join(".x07_rr/http")).expect("mkdir");
        std::fs::write(
            root.join(".x07_rr/http/002-response.json"),
            r#"{"at":"2","kind":"os-net","policy":"http","summary":"response"}"#,
        )
        .expect("second");
        std::fs::write(
            root.join(".x07_rr/http/001-request.json"),
            r#"{"at":"1","kind":"os-net","policy":"http","summary":"request"}"#,
        )
        .expect("first");

        let ribbon = ribbon(root.as_path());

        assert_eq!(ribbon.boundaries.len(), 2);
        assert!(ribbon.boundaries[0]
            .cassette_path
            .ends_with("001-request.json"));
        std::fs::remove_dir_all(root).ok();
    }
}

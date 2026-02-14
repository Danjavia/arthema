use crate::collections::ApiRequest;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn parse_bruno_folder(dir_path: &Path) -> Vec<ApiRequest> {
    let mut requests = Vec::new();
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                requests.extend(parse_bruno_folder(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("bru") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(req) = parse_bru_file(&content) {
                        let group = path.parent()
                            .and_then(|p| p.file_name())
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string());
                        
                        let mut req_with_group = req;
                        req_with_group.group = group;
                        requests.push(req_with_group);
                    }
                }
            }
        }
    }
    requests
}

fn parse_bru_file(content: &str) -> Option<ApiRequest> {
    let mut name = String::from("Unnamed Bruno");
    let mut url = String::new();
    let mut method = String::from("GET");
    let mut body = String::new();
    let mut in_body = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name:") {
            name = trimmed.replace("name:", "").trim().to_string();
        } else if trimmed.ends_with("{") {
            let m = trimmed.split_whitespace().next().unwrap_or("GET").to_uppercase();
            if ["GET", "POST", "PUT", "DELETE", "PATCH"].contains(&m.as_str()) {
                method = m;
            }
            if trimmed.starts_with("body") { in_body = true; }
        } else if trimmed.starts_with("url:") {
            url = trimmed.replace("url:", "").trim().to_string();
        } else if trimmed == "}" {
            in_body = false;
        } else if in_body {
            body.push_str(line);
            body.push('\n');
        }
    }

    if url.is_empty() { return None; }

    Some(ApiRequest {
        name,
        url,
        method,
        headers: HashMap::new(),
        body: if body.trim().is_empty() { None } else { Some(body.trim().to_string()) },
        group: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bru_file() {
        let content = r#"
meta {
  name: Get Users
  type: http
}
get {
  url: https://api.com/users
}
"#;
        let req = parse_bru_file(content).unwrap();
        assert_eq!(req.name, "Get Users");
        assert_eq!(req.url, "https://api.com/users");
    }
}

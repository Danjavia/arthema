use std::collections::HashMap;

pub struct ParsedCurl {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

pub fn parse_curl(curl: &str) -> Option<ParsedCurl> {
    let trimmed = curl.trim();
    if !trimmed.to_lowercase().starts_with("curl") {
        return None;
    }

    let mut method = String::new();
    let mut url = String::new();
    let mut headers = HashMap::new();
    let mut body_parts = Vec::new();
    
    // Limpiar saltos de línea y escapar caracteres de shell
    let cleaned_curl = trimmed.replace("\\\n", " ").replace("\\\r\n", " ");
    
    let tokens = shlex::split(&cleaned_curl)?;
    let mut iter = tokens.iter().peekable();
    iter.next(); // saltar "curl"

    while let Some(token) = iter.next() {
        match token.as_str() {
            "-X" | "--request" => {
                if let Some(m) = iter.next() {
                    method = m.to_uppercase();
                }
            }
            "-H" | "--header" => {
                if let Some(h) = iter.next() {
                    if let Some((k, v)) = h.split_once(':') {
                        headers.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                if let Some(d) = iter.next() {
                    body_parts.push(d.clone());
                    if method.is_empty() {
                        method = "POST".to_string();
                    }
                }
            }
            u if u.starts_with("http") => {
                url = u.to_string();
            }
            flag if flag.starts_with('-') => {
                // Otras flags que no nos interesan por ahora
                if !["-L", "--location", "-i", "--include", "-s", "--silent"].contains(&flag) {
                    // Si es una flag que espera valor y no la conocemos, saltamos el siguiente
                    // Pero por ahora, el parser simple de arriba cubre lo básico.
                }
            }
            _ => {
                if url.is_empty() {
                    url = token.to_string();
                }
            }
        }
    }

    if url.is_empty() { return None; }
    if method.is_empty() { method = "GET".to_string(); }

    Some(ParsedCurl {
        method,
        url,
        headers,
        body: if body_parts.is_empty() { None } else { Some(body_parts.join("")) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let curl = "curl https://api.example.com/data";
        let parsed = parse_curl(curl).unwrap();
        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.url, "https://api.example.com/data");
    }

    #[test]
    fn test_parse_post_with_headers_and_body() {
        let curl = "curl -X POST https://api.com -H 'Content-Type: application/json' -d '{\"key\":\"val\"}'";
        let parsed = parse_curl(curl).unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.headers.get("Content-Type").unwrap(), "application/json");
        assert_eq!(parsed.body.unwrap(), "{\"key\":\"val\"}");
    }

    #[test]
    fn test_parse_multiline_curl() {
        let curl = "curl -X PUT https://api.com \\\n -H 'Authorization: Bearer 123' \\\n -d 'data'";
        let parsed = parse_curl(curl).unwrap();
        assert_eq!(parsed.method, "PUT");
        assert_eq!(parsed.headers.get("Authorization").unwrap(), "Bearer 123");
        assert_eq!(parsed.body.unwrap(), "data");
    }
}

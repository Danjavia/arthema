use std::collections::HashMap;

pub struct ParsedCurl {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

pub fn parse_curl(curl: &str) -> Option<ParsedCurl> {
    if !curl.trim().to_lowercase().starts_with("curl") {
        return None;
    }

    let mut method = "GET".to_string();
    let mut url = String::new();
    let mut headers = HashMap::new();
    let mut body = String::new();
    
    // Unir líneas si el curl está multilínea con 
    let cleaned_curl = curl.replace("
", " ").replace("
", " ");
    
    // Split por espacios pero respetando comillas (simplificado)
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
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                if let Some(d) = iter.next() {
                    body.push_str(d);
                    if method == "GET" {
                        method = "POST".to_string();
                    }
                }
            }
            u if u.starts_with("http") => {
                url = u.to_string();
            }
            _ => {
                // Si no empieza con - y no es una flag conocida, podría ser la URL
                if !token.starts_with('-') && url.is_empty() {
                    url = token.to_string();
                }
            }
        }
    }

    if url.is_empty() { return None; }

    Some(ParsedCurl {
        method,
        url,
        headers,
        body: if body.is_empty() { None } else { Some(body) },
    })
}

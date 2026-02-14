use serde_json::Value;
use crate::collections::ApiRequest;
use std::collections::HashMap;

pub fn parse_postman(content: &str) -> Vec<ApiRequest> {
    let json: Value = serde_json::from_str(content).unwrap_or(Value::Null);
    let mut requests = Vec::new();
    
    if let Some(items) = json["item"].as_array() {
        process_items(items, None, &mut requests);
    } else if let Some(items) = json.as_array() {
        // A veces Postman exporta un array directo
        process_items(items, None, &mut requests);
    }
    
    requests
}

fn process_items(items: &[Value], group: Option<String>, requests: &mut Vec<ApiRequest>) {
    for item in items {
        // Caso 1: Es una carpeta (tiene campo 'item')
        if let Some(sub_items) = item["item"].as_array() {
            let folder_name = item["name"].as_str().unwrap_or("Folder").to_string();
            // Mantener jerarquía simple uniendo nombres si es necesario o solo el último tag
            process_items(sub_items, Some(folder_name), requests);
        } 
        // Caso 2: Es una petición (tiene campo 'request')
        else if item.get("request").is_some() {
            let req_obj = &item["request"];
            let name = item["name"].as_str().unwrap_or("Unnamed").to_string();
            
            let method = if req_obj.is_string() {
                "GET".to_string() // Postman a veces simplifica
            } else {
                req_obj["method"].as_str().unwrap_or("GET").to_string()
            };
            
            let url = if req_obj.is_string() {
                req_obj.as_str().unwrap().to_string()
            } else if let Some(u) = req_obj.get("url") {
                if u.is_string() { u.as_str().unwrap().to_string() }
                else { u["raw"].as_str().unwrap_or("").to_string() }
            } else { "".to_string() };

            let mut headers = HashMap::new();
            if let Some(h_list) = req_obj.get("header").and_then(|h| h.as_array()) {
                for h in h_list {
                    if let (Some(k), Some(v)) = (h["key"].as_str(), h["value"].as_str()) {
                        headers.insert(k.to_string(), v.to_string());
                    }
                }
            }

            let body = req_obj.get("body")
                .and_then(|b| b.get("raw"))
                .and_then(|r| r.as_str())
                .map(|s| s.to_string());

            requests.push(ApiRequest {
                name,
                url,
                method,
                headers,
                body,
                group: group.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_postman_collection() {
        let json = r#"{
            "item": [
                {
                    "name": "Auth",
                    "item": [
                        {
                            "name": "Login",
                            "request": {
                                "method": "POST",
                                "url": "https://api.com/login",
                                "header": [{"key": "Content-Type", "value": "application/json"}],
                                "body": {"raw": "{\"user\":\"test\"}"}
                            }
                        }
                    ]
                }
            ]
        }"#;
        let reqs = parse_postman(json);
        assert!(reqs.len() > 0, "Debería haber al menos una petición");
        assert_eq!(reqs[0].name, "Login");
        assert_eq!(reqs[0].group.as_ref().unwrap(), "Auth");
    }
}

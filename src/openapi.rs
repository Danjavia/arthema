use openapiv3::OpenAPI;
use crate::collections::ApiRequest;
use std::collections::HashMap;

pub fn parse_swagger(content: &str) -> Vec<ApiRequest> {
    let spec: OpenAPI = if let Ok(s) = serde_json::from_str(content) {
        s
    } else if let Ok(s) = serde_yaml::from_str(content) {
        s
    } else {
        return Vec::new();
    };

    let mut requests = Vec::new();
    let base_url = spec.servers.first().map(|s| s.url.clone()).unwrap_or_else(|| "http://localhost".to_string());

    for (path, item) in spec.paths.iter() {
        if let Some(ref path_item) = item.as_item() {
            // Helper para procesar cada operación
            let mut process_op = |method: &str, op: &openapiv3::Operation| {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());

                // Intentar extraer el grupo de los tags
                let group = op.tags.first().cloned();
                let name = op.summary.clone().unwrap_or_else(|| format!("{} {}", method, path));

                requests.push(ApiRequest {
                    name,
                    url: format!("{}{}", base_url, path),
                    method: method.to_string(),
                    headers,
                    body: None, // Por ahora simplificado, se puede mejorar con examples
                    group,
                });
            };

            if let Some(ref op) = path_item.get { process_op("GET", op); }
            if let Some(ref op) = path_item.post { process_op("POST", op); }
            if let Some(ref op) = path_item.put { process_op("PUT", op); }
            if let Some(ref op) = path_item.delete { process_op("DELETE", op); }
            if let Some(ref op) = path_item.patch { process_op("PATCH", op); }
        }
    }

    requests
}

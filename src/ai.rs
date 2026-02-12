use serde_json::json;
use reqwest::Client;

pub async fn get_ai_suggestion(prompt: &str) -> String {
    let api_key = "AIzaSyCFAUvm0c4jdJ8Pl-xB6S7eGEEjFs40b0c";
    let system_prompt = "You are Arthema AI. Suggest ONE cool public API URL based on this hint: '{}'. Return ONLY the URL string.";
    call_gemini(api_key, system_prompt, prompt).await
}

pub async fn explain_response(response: &str) -> String {
    let api_key = "AIzaSyCFAUvm0c4jdJ8Pl-xB6S7eGEEjFs40b0c";
    let truncated_res = if response.len() > 3000 { &response[..3000] } else { response };
    let system_prompt = "You are Arthema AI. Analyze this API response technicaly and concisely.";
    call_gemini(api_key, system_prompt, truncated_res).await
}

pub async fn fix_error(method: &str, url: &str, headers: &str, body: &str, error: &str) -> String {
    let api_key = "AIzaSyCFAUvm0c4jdJ8Pl-xB6S7eGEEjFs40b0c";
    let prompt = format!(
        "The following API request failed.\nMethod: {}\nURL: {}\nHeaders: {}\nBody: {}\nError Received: {}\n\nSuggest a fix for the headers or body. Be extremely technical and concise.",
        method, url, headers, body, error
    );
    let system_prompt = "You are Arthema AI Fixer. Identify why the request failed and suggest the exact change needed.";
    call_gemini(api_key, system_prompt, &prompt).await
}

async fn call_gemini(api_key: &str, system_prompt: &str, user_input: &str) -> String {
    let client = Client::new();
    // Usando explícitamente el modelo gemini-2.5-flash-lite solicitado
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent?key={}",
        api_key
    );

    let body = json!({
        "contents": [{
            "parts": [{
                "text": format!("{}\n\nInput: {}", system_prompt, user_input)
            }]
        }]
    });

    match client.post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() == 429 {
                    return "AI ERROR: Rate limit exceeded (429). Please wait a minute before next AI request.".to_string();
                }
                if !status.is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    return format!("AI ERROR: Status {}. Body: {}", status, err_text);
                }
                
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                json["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or("SYSTEM ERROR: Neural link returned empty data stream")
                    .trim()
                    .to_string()
            },
            Err(e) => format!("AI CONNECTION LOST: {}", e),
        }
}

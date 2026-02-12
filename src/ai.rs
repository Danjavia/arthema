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
    let system_prompt = "You are Arthema AI. Analyze this API response. Be concise, technical, and use a cyberpunk tone.";
    
    call_gemini(api_key, system_prompt, truncated_res).await
}

async fn call_gemini(api_key: &str, system_prompt: &str, user_input: &str) -> String {
    let client = Client::new();
    // Usando explícitamente el modelo gemini-2.0-flash-lite
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent?key={}",
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
                if !status.is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    return format!("AI LINK ERROR ({}): {}", status, err_text);
                }
                
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let text = json["candidates"][0]["content"]["parts"][0]["text"].as_str();
                
                match text {
                    Some(t) => t.trim().to_string(),
                    None => "SYSTEM ERROR: Neural link returned empty data stream".to_string()
                }
            },
            Err(e) => format!("AI CONNECTION LOST: {}", e),
        }
}

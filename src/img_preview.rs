use base64::{Engine as _, engine::general_purpose};

pub fn generate_hifi_preview(bytes: &[u8], width: u32) -> String {
    // Si estamos en iTerm2, usamos su protocolo de imágenes
    let term = std::env::var("TERM_PROGRAM").unwrap_or_default();
    
    if term == "iTerm.app" {
        let b64_data = general_purpose::STANDARD.encode(bytes);
        // Protocolo iTerm2: ^]1337;File=inline=1;width=WIDTHpx:BASE64_DATA^G
        // Ajustamos el width al ancho del panel (cada carácter mide aprox 10px de ancho)
        let pixel_width = width * 8; 
        format!(
            "\x1b]1337;File=inline=1;width={}px;preserveAspectRatio=1:{}\x07\n",
            pixel_width, b64_data
        )
    } else {
        // Fallback para otras terminales
        "🖼️ [Binary Image Data]\n\nTerminal does not support inline images.\nPress 'o' to open in System Preview.".to_string()
    }
}

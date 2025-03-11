use colored::*;
use reqwest::blocking::Client;
use serde_json::Value;

/// Corrige la ortografía de un texto utilizando la API de Groq
pub fn correct_spelling(text: &str) -> String {
    // Si el texto está vacío, devolverlo tal cual
    if text.trim().is_empty() {
        return text.to_string();
    }

    // Obtener la clave API de Groq desde una variable de entorno
    let api_key = match std::env::var("GROQ_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!(
                "{}",
                "No se encontró la clave API de Groq. Usando texto original.".yellow()
            );
            return text.to_string();
        }
    };

    // Crear cliente HTTP
    let client = Client::new();

    // Crear el cuerpo de la solicitud para la API de Groq
    let request_body = serde_json::json!({
        "messages": [
            {
                "role": "system",
                "content": "Eres un asistente especializado en corrección ortográfica y gramatical en español. Tu tarea es corregir errores ortográficos y gramaticales en el texto proporcionado, manteniendo el significado original. Solo debes devolver el texto corregido, sin explicaciones ni comentarios adicionales."
            },
            {
                "role": "user",
                "content": format!("Corrige la ortografía y gramática del siguiente texto, manteniendo su significado original: \"{}\"", text)
            }
        ],
        "model": "llama3-8b-8192"
    });

    // Realizar la solicitud a la API de Groq
    match client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
    {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                match response.json::<Value>() {
                    Ok(json) => {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            if let Some(first_choice) = choices.first() {
                                if let Some(message) = first_choice.get("message") {
                                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                        println!(
                                            "{}",
                                            "Corrección ortográfica completada.".green()
                                        );
                                        return content.to_string();
                                    }
                                }
                            }
                        }
                        println!(
                            "{}",
                            "No se pudo obtener la respuesta de la API de Groq. Usando texto original.".yellow()
                        );
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            format!("Error al procesar la respuesta de la API de Groq: {}. Usando texto original.", e).red()
                        );
                    }
                }
            } else {
                match response.json::<Value>() {
                    Ok(error_json) => {
                        if let Some(error) = error_json.get("error").and_then(|e| e.as_object()) {
                            let message = error
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Error desconocido");
                            println!("{}", format!("Error de la API de Groq: {}", message).red());
                        } else {
                            println!("{}", format!("Error de la API de Groq: {}", status).red());
                        }
                    }
                    Err(_) => {
                        println!("{}", format!("Error de la API de Groq: {}", status).red());
                    }
                }
            }
        }
        Err(e) => {
            println!(
                "{}",
                format!("Error al conectar con la API de Groq: {}", e).red()
            );
        }
    }

    // En caso de error, devolver el texto original
    println!(
        "{}",
        "No se pudo corregir el texto. Usando texto original.".yellow()
    );
    text.to_string()
}

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use chrono::Local;
use colored::*;
use csv::{Reader, Writer};
use serde_json::Value;

use crate::models::TestCase;

/// Carga casos de prueba desde un archivo CSV
pub fn load_from_csv(file_path: &str) -> io::Result<Vec<TestCase>> {
    let mut test_cases = Vec::new();

    // Verificar si el archivo existe
    if !Path::new(file_path).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("El archivo {} no existe", file_path),
        ));
    }

    let file = File::open(file_path)?;
    let mut reader = Reader::from_reader(file);

    for result in reader.deserialize() {
        match result {
            Ok(test_case) => test_cases.push(test_case),
            Err(e) => println!("{}", format!("Error al leer caso de prueba: {}", e).red()),
        }
    }

    Ok(test_cases)
}

/// Guarda casos de prueba en un archivo CSV
pub fn save_to_csv(file_path: &str, test_cases: &[TestCase]) -> io::Result<()> {
    let file = File::create(file_path)?;
    let mut writer = Writer::from_writer(file);

    for test_case in test_cases {
        writer.serialize(test_case)?;
    }

    writer.flush()?;

    Ok(())
}

/// Guarda casos de prueba en formato Markdown
pub fn save_to_markdown(file_path: &str, test_cases: &[TestCase], title: &str) -> io::Result<()> {
    let mut file = File::create(file_path)?;

    // Escribir encabezado
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "# Informe de Pruebas: {}", title)?;
    writeln!(file, "\nFecha de ejecución: {}", timestamp)?;
    
    // Añadir versión y tickets si hay casos de prueba disponibles
    if let Some(first_case) = test_cases.first() {
        writeln!(file, "Versión de prueba: {}", first_case.version)?;
        let ticket_info = if first_case.ticket_numbers.is_empty() {
            "N/A".to_string()
        } else {
            first_case.ticket_numbers.clone()
        };
        writeln!(file, "Ticket(s): {}\n", ticket_info)?;
    } else {
        writeln!(file, "")?;
    }

    // Calcular resumen
    let validated = test_cases
        .iter()
        .filter(|tc| tc.status == crate::models::TestStatus::Validated)
        .count();
    let rejected = test_cases
        .iter()
        .filter(|tc| tc.status == crate::models::TestStatus::Rejected)
        .count();
    let pending = test_cases
        .iter()
        .filter(|tc| tc.status == crate::models::TestStatus::Pending)
        .count();
    let skipped = test_cases
        .iter()
        .filter(|tc| tc.status == crate::models::TestStatus::Skipped)
        .count();
    let blocked = test_cases
        .iter()
        .filter(|tc| tc.status == crate::models::TestStatus::Blocked)
        .count();

    // Escribir resumen textual primero
    writeln!(file, "## Resumen Numérico\n")?;
    writeln!(file, "- Total de casos: {}", test_cases.len())?;
    writeln!(file, "- ✅ Validados: {}", validated)?;
    writeln!(file, "- ❌ Rechazados: {}", rejected)?;
    writeln!(file, "- ⏳ Pendientes: {}", pending)?;
    writeln!(file, "- ⏭️ Omitidos: {}", skipped)?;
    writeln!(file, "- 🚫 Bloqueados: {}\n", blocked)?;

    // Crear gráfico circular con Mermaid
    writeln!(file, "## Resumen Visual\n")?;
    writeln!(file, "```mermaid")?;
    writeln!(file, "pie title Distribución de Casos de Prueba")?;

    // Añadir secciones al gráfico solo si tienen valores mayores que cero
    if validated > 0 {
        writeln!(file, "    \"✅ Validados\" : {}", validated)?; // Verde
    }
    if rejected > 0 {
        writeln!(file, "    \"❌ Rechazados\" : {}", rejected)?; // Rojo
    }
    if pending > 0 {
        writeln!(file, "    \"⏳ Pendientes\" : {}", pending)?; // Amarillo
    }
    if skipped > 0 {
        writeln!(file, "    \"⏭️ Omitidos\" : {}", skipped)?; // Azul
    }
    if blocked > 0 {
        writeln!(file, "    \"🚫 Bloqueados\" : {}", blocked)?; // Gris
    }
    writeln!(file, "```\n")?;

    // Escribir detalles de cada caso de prueba
    writeln!(file, "## Detalle de Casos de Prueba\n")?;
    for (i, test_case) in test_cases.iter().enumerate() {
        writeln!(file, "### {}. {}", i + 1, test_case.description)?;
        writeln!(file, "- **Estado**: {}", test_case.status)?;
        
        // Solo mostrar observaciones si no están vacías
        if !test_case.observations.is_empty() {
            writeln!(file, "- **Observaciones**: {}", test_case.observations)?;
        }
        
        // Solo mostrar evidencia si no está vacía
        if !test_case.evidence.is_empty() {
            writeln!(file, "- **Evidencia**: {}", test_case.evidence)?;
        }
        
        writeln!(file, "")?;
    }

    Ok(())
}

/// Obtiene la lista de archivos de definición disponibles
pub fn get_definition_files() -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    
    // Verificar si el directorio existe
    if !Path::new("definitions").exists() {
        fs::create_dir("definitions")?;
        return Ok(files);
    }
    
    for entry in fs::read_dir("definitions")? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "csv") {
            if let Some(path_str) = path.to_str() {
                files.push(path_str.to_string());
            }
        }
    }
    
    // Ordenar alfabéticamente
    files.sort();
    
    Ok(files)
}

/// Obtiene la lista de archivos de ejecución disponibles
pub fn get_execution_files() -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    
    // Verificar si el directorio existe
    if !Path::new("executions").exists() {
        fs::create_dir("executions")?;
        return Ok(files);
    }
    
    for entry in fs::read_dir("executions")? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "csv") {
            if let Some(path_str) = path.to_str() {
                files.push(path_str.to_string());
            }
        }
    }
    
    // Ordenar alfabéticamente
    files.sort();
    
    Ok(files)
}

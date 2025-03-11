use clap::{Parser, Subcommand};
use chrono::Local;
use colored::*;
use csv::Writer;
use inquire::{Select, Text};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, create_dir_all};
use std::io::{self, Write};
use std::path::Path;
use uuid::Uuid;
use reqwest::blocking::Client;
use serde_json::Value;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Crear nuevos casos de prueba
    Create {
        /// Tipo de prueba: smoke o regression
        #[arg(short, long)]
        test_type: String,
        
        /// Nombre del archivo de prueba
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Modificar casos de prueba existentes
    Modify {
        /// Ruta al archivo CSV de prueba
        #[arg(short, long)]
        file: String,
    },
    /// Ejecutar casos de prueba
    Execute {
        /// Ruta al archivo CSV de prueba
        #[arg(short, long)]
        file: String,
    },
    /// Listar archivos de prueba disponibles
    List,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TestCase {
    id: String,
    description: String,
    status: TestStatus,
    observations: String,
    evidence: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum TestStatus {
    Pending,
    Validated,
    Rejected,
    Skipped,
    Blocked,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Pending => write!(f, "⏳ Pendiente"),
            TestStatus::Validated => write!(f, "✅ Validado"),
            TestStatus::Rejected => write!(f, "❌ Rechazado"),
            TestStatus::Skipped => write!(f, "⏭️ Omitido"),
            TestStatus::Blocked => write!(f, "🚫 Bloqueado"),
        }
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    
    // Crear directorio para almacenar pruebas si no existe
    let test_dir = Path::new("tests");
    if !test_dir.exists() {
        create_dir_all(test_dir)?;
    }
    
    match &cli.command {
        Some(Commands::Create { test_type, name }) => {
            create_test_cases(test_type, name.clone())?
        },
        Some(Commands::Modify { file }) => {
            modify_test_cases(file)?
        },
        Some(Commands::Execute { file }) => {
            execute_test_cases(file)?
        },
        Some(Commands::List) => {
            list_test_files()?
        },
        None => {
            // Menú interactivo si no se proporciona un comando
            let options = vec!["Crear casos de prueba", "Modificar casos de prueba", "Ejecutar casos de prueba", "Listar archivos de prueba", "Salir"];
            
            let selection = Select::new("¿Qué deseas hacer?", options)
                .prompt();
            
            match selection {
                Ok("Crear casos de prueba") => {
                    let test_types = vec!["smoke", "regression"];
                    let test_type = Select::new("Selecciona el tipo de prueba:", test_types)
                        .prompt()
                        .unwrap_or("smoke");
                    
                    let name = Text::new("Nombre del archivo (opcional):")
                        .prompt()
                        .ok();
                    
                    create_test_cases(test_type, name)?
                },
                Ok("Modificar casos de prueba") => {
                    let file = select_test_file()?;
                    if let Some(file_path) = file {
                        modify_test_cases(&file_path)?
                    }
                },
                Ok("Ejecutar casos de prueba") => {
                    let file = select_test_file()?;
                    if let Some(file_path) = file {
                        execute_test_cases(&file_path)?
                    }
                },
                Ok("Listar archivos de prueba") => {
                    list_test_files()?
                },
                _ => println!("¡Hasta pronto!"),
            }
        },
    }
    
    Ok(())
}

/// Selecciona un archivo de prueba existente
fn select_test_file() -> io::Result<Option<String>> {
    let test_files = get_test_files()?;
    
    if test_files.is_empty() {
        println!("{}", "No hay archivos de prueba disponibles.".red());
        return Ok(None);
    }
    
    let selection = Select::new("Selecciona un archivo de prueba:", test_files)
        .prompt();
    
    match selection {
        Ok(file) => Ok(Some(file)),
        Err(_) => Ok(None),
    }
}

/// Obtiene la lista de archivos de prueba CSV disponibles
fn get_test_files() -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    let test_dir = Path::new("tests");
    
    if !test_dir.exists() {
        return Ok(files);
    }
    
    for entry in fs::read_dir(test_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "csv") {
            if let Some(path_str) = path.to_str() {
                files.push(path_str.to_string());
            }
        }
    }
    
    Ok(files)
}

/// Lista los archivos de prueba disponibles
fn list_test_files() -> io::Result<()> {
    let files = get_test_files()?;
    
    if files.is_empty() {
        println!("{}", "No hay archivos de prueba disponibles.".yellow());
        return Ok(());
    }
    
    println!("{}", "Archivos de prueba disponibles:".green());
    for (i, file) in files.iter().enumerate() {
        println!("{}: {}", i + 1, file);
    }
    
    Ok(())
}

/// Crea nuevos casos de prueba
fn create_test_cases(test_type: &str, name: Option<String>) -> io::Result<()> {
    // Validar tipo de prueba
    if test_type != "smoke" && test_type != "regression" {
        println!("{}", "Tipo de prueba inválido. Use 'smoke' o 'regression'.".red());
        return Ok(());
    }
    
    // Generar nombre de archivo
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let file_name = match name {
        Some(n) if !n.is_empty() => format!("{}-{}-{}", test_type, n, timestamp),
        _ => format!("{}-{}", test_type, timestamp),
    };
    
    let csv_path = format!("tests/{}.csv", file_name);
    let md_path = format!("tests/{}.md", file_name);
    
    // Crear casos de prueba
    let mut test_cases = Vec::new();
    let mut i = 1;
    
    println!("{}", "Ingresa los casos de prueba. Escribe 'FIN' en la descripción para terminar.".blue());
    
    loop {
        println!("{}", format!("Caso de prueba #{}", i).blue());
        
        let id = Uuid::new_v4().to_string().split('-').next().unwrap_or("TC").to_string();
        
        let description = Text::new("Descripción (o escribe 'FIN' para terminar):")
            .prompt()
            .unwrap_or_else(|_| format!("Caso de prueba {}", i));
        
        // Verificar si el usuario quiere terminar
        if description.trim().to_uppercase() == "FIN" {
            break;
        }
        
        let test_case = TestCase {
            id,
            description,
            status: TestStatus::Pending,
            observations: String::new(),
            evidence: String::new(),
        };
        
        test_cases.push(test_case);
        i += 1;
    }
    
    // Verificar si se creó al menos un caso de prueba
    if test_cases.is_empty() {
        println!("{}", "No se crearon casos de prueba.".yellow());
        return Ok(());
    }
    
    // Preguntar si desea corregir la ortografía
    let options = vec!["Sí", "No"];
    let selection = Select::new("¿Deseas corregir la ortografía de las descripciones?", options)
        .prompt();
    
    if let Ok("Sí") = selection {
        println!("{}", "Corrigiendo ortografía...".blue());
        
        // Corregir ortografía de las descripciones
        let mut corrected_cases = Vec::new();
        
        for mut test_case in test_cases {
            let corrected_description = correct_spelling(&test_case.description);
            
            // Solo actualizar si hay cambios
            if corrected_description != test_case.description {
                println!("{}", format!("Corrección: '{}' -> '{}'", test_case.description, corrected_description).green());
                test_case.description = corrected_description;
            }
            
            corrected_cases.push(test_case);
        }
        
        // Guardar en CSV
        save_to_csv(&csv_path, &corrected_cases)?;
        
        // Guardar en Markdown
        save_to_markdown(&md_path, &corrected_cases, &file_name)?;
    } else {
        // Guardar en CSV sin corrección
        save_to_csv(&csv_path, &test_cases)?;
        
        // Guardar en Markdown sin corrección
        save_to_markdown(&md_path, &test_cases, &file_name)?;
    }
    
    println!("{}", format!("Casos de prueba creados y guardados en {} y {}", csv_path, md_path).green());
    
    Ok(())
}

/// Modifica casos de prueba existentes
fn modify_test_cases(file_path: &str) -> io::Result<()> {
    let mut test_cases = load_from_csv(file_path)?;
    
    if test_cases.is_empty() {
        println!("{}", "No hay casos de prueba para modificar.".yellow());
        return Ok(());
    }
    
    // Mostrar casos de prueba
    println!("{}", "Casos de prueba disponibles:".blue());
    for (i, test_case) in test_cases.iter().enumerate() {
        println!("{}: {} - {} - {}", i + 1, test_case.id, test_case.description, test_case.status);
    }
    
    // Seleccionar caso de prueba a modificar
    let selection = Select::new(
        "Selecciona un caso de prueba para modificar:",
        (0..test_cases.len()).map(|i| format!("{}: {}", i + 1, test_cases[i].description)).collect::<Vec<_>>()
    ).prompt();
    
    match selection {
        Ok(selected) => {
            let index = selected.split(':').next().unwrap_or("1").parse::<usize>().unwrap_or(1) - 1;
            
            if index < test_cases.len() {
                let options = vec!["Descripción", "Estado", "Observaciones", "Evidencia"];
                
                let field = Select::new("¿Qué campo deseas modificar?", options)
                    .prompt()
                    .unwrap_or("Estado");
                
                match field {
                    "Descripción" => {
                        let new_description = Text::new("Nueva descripción:")
                            .with_initial_value(&test_cases[index].description)
                            .prompt()
                            .unwrap_or_else(|_| test_cases[index].description.clone());
                        
                        test_cases[index].description = new_description;
                    },
                    "Estado" => {
                        let status_options = vec!["⏳ Pendiente", "✅ Validado", "❌ Rechazado", "⏭️ Omitido", "🚫 Bloqueado"];
                        
                        let new_status = Select::new("Nuevo estado:", status_options)
                            .prompt()
                            .unwrap_or("⏳ Pendiente");
                        
                        test_cases[index].status = match new_status {
                            "✅ Validado" => TestStatus::Validated,
                            "❌ Rechazado" => TestStatus::Rejected,
                            "⏭️ Omitido" => TestStatus::Skipped,
                            "🚫 Bloqueado" => TestStatus::Blocked,
                            _ => TestStatus::Pending,
                        };
                    },
                    "Observaciones" => {
                        let new_observations = Text::new("Nuevas observaciones:")
                            .with_initial_value(&test_cases[index].observations)
                            .prompt()
                            .unwrap_or_else(|_| test_cases[index].observations.clone());
                        
                        test_cases[index].observations = new_observations;
                    },
                    "Evidencia" => {
                        let new_evidence = Text::new("Nueva evidencia (ruta o URL):")
                            .with_initial_value(&test_cases[index].evidence)
                            .prompt()
                            .unwrap_or_else(|_| test_cases[index].evidence.clone());
                        
                        test_cases[index].evidence = new_evidence;
                    },
                    _ => {},
                }
                
                // Guardar cambios
                save_to_csv(file_path, &test_cases)?;
                
                // Actualizar archivo markdown
                let md_path = file_path.replace(".csv", ".md");
                let file_name = Path::new(file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("test_cases");
                
                save_to_markdown(&md_path, &test_cases, file_name)?;
                
                println!("{}", "Caso de prueba modificado correctamente.".green());
            }
        },
        Err(_) => println!("{}", "Operación cancelada.".yellow()),
    }
    
    Ok(())
}

/// Ejecuta casos de prueba
fn execute_test_cases(file_path: &str) -> io::Result<()> {
    let mut test_cases = load_from_csv(file_path)?;
    
    if test_cases.is_empty() {
        println!("{}", "No hay casos de prueba para ejecutar.".yellow());
        return Ok(());
    }
    
    println!("{}", format!("Ejecutando casos de prueba de {}", file_path).blue());
    
    for (i, test_case) in test_cases.iter_mut().enumerate() {
        println!("{}", format!("Caso de prueba #{}: {}", i + 1, test_case.description).blue());
        
        // Mostrar estado actual
        println!("Estado actual: {}", test_case.status);
        
        // Seleccionar nuevo estado
        let status_options = vec!["⏳ Pendiente", "✅ Validado", "❌ Rechazado", "⏭️ Omitido", "🚫 Bloqueado"];
        
        let new_status = Select::new("Selecciona el resultado de la ejecución:", status_options)
            .prompt()
            .unwrap_or("⏳ Pendiente");
        
        test_case.status = match new_status {
            "✅ Validado" => TestStatus::Validated,
            "❌ Rechazado" => TestStatus::Rejected,
            "⏭️ Omitido" => TestStatus::Skipped,
            "🚫 Bloqueado" => TestStatus::Blocked,
            _ => TestStatus::Pending,
        };
        
        // Agregar observaciones
        let observations = Text::new("Observaciones (opcional):")
            .with_initial_value(&test_case.observations)
            .prompt()
            .unwrap_or_else(|_| test_case.observations.clone());
        
        test_case.observations = observations;
        
        // Agregar evidencia
        let evidence = Text::new("Evidencia (ruta o URL, opcional):")
            .with_initial_value(&test_case.evidence)
            .prompt()
            .unwrap_or_else(|_| test_case.evidence.clone());
        
        test_case.evidence = evidence;
    }
    
    // Guardar cambios
    save_to_csv(file_path, &test_cases)?;
    
    // Actualizar archivo markdown
    let md_path = file_path.replace(".csv", ".md");
    let file_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("test_cases");
    
    save_to_markdown(&md_path, &test_cases, file_name)?;
    
    println!("{}", "Ejecución de casos de prueba completada y guardada.".green());
    
    Ok(())
}

/// Carga casos de prueba desde un archivo CSV
fn load_from_csv(file_path: &str) -> io::Result<Vec<TestCase>> {
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            println!("{}", format!("Error al abrir el archivo: {}", e).red());
            return Ok(Vec::new());
        }
    };
    
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    
    let mut test_cases = Vec::new();
    
    for result in reader.deserialize() {
        match result {
            Ok(test_case) => test_cases.push(test_case),
            Err(e) => println!("{}", format!("Error al leer caso de prueba: {}", e).red()),
        }
    }
    
    Ok(test_cases)
}

/// Guarda casos de prueba en un archivo CSV
fn save_to_csv(file_path: &str, test_cases: &[TestCase]) -> io::Result<()> {
    let file = File::create(file_path)?;
    let mut writer = Writer::from_writer(file);
    
    for test_case in test_cases {
        writer.serialize(test_case)?;
    }
    
    writer.flush()?;
    
    Ok(())
}

/// Guarda casos de prueba en un archivo Markdown
/// Corrige la ortografía de un texto utilizando la API de LanguageTool
fn correct_spelling(text: &str) -> String {
    // Si el texto está vacío, devolverlo tal cual
    if text.trim().is_empty() {
        return text.to_string();
    }
    
    // Crear cliente HTTP
    let client = Client::new();
    
    // Parámetros para la API de LanguageTool
    let params = [
        ("text", text),
        ("language", "es"),  // Español
        ("enabledOnly", "false"),
    ];
    
    // Intentar hacer la solicitud a la API
    match client.post("https://api.languagetool.org/v2/check")
        .form(&params)
        .send() {
            Ok(response) => {
                // Verificar si la respuesta es exitosa
                if response.status().is_success() {
                    // Intentar parsear la respuesta JSON
                    match response.json::<Value>() {
                        Ok(json) => {
                            // Obtener las correcciones
                            if let Some(matches) = json.get("matches").and_then(|m| m.as_array()) {
                                let mut corrected = text.to_string();
                                
                                // Aplicar correcciones de atrás hacia adelante para no afectar los índices
                                let mut corrections: Vec<(usize, usize, String)> = Vec::new();
                                
                                for m in matches {
                                    if let (Some(offset), Some(length), Some(replacements)) = (
                                        m.get("offset").and_then(|o| o.as_u64()),
                                        m.get("length").and_then(|l| l.as_u64()),
                                        m.get("replacements").and_then(|r| r.as_array())
                                    ) {
                                        // Tomar la primera sugerencia si existe
                                        if let Some(first_replacement) = replacements.first() {
                                            if let Some(value) = first_replacement.get("value").and_then(|v| v.as_str()) {
                                                corrections.push((offset as usize, length as usize, value.to_string()));
                                            }
                                        }
                                    }
                                }
                                
                                // Ordenar las correcciones de mayor a menor offset
                                corrections.sort_by(|a, b| b.0.cmp(&a.0));
                                
                                // Aplicar correcciones
                                for (offset, length, replacement) in corrections {
                                    if offset + length <= corrected.len() {
                                        corrected.replace_range(offset..(offset + length), &replacement);
                                    }
                                }
                                
                                return corrected;
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(_) => {}
    }
    
    // En caso de error, devolver el texto original
    text.to_string()
}

fn save_to_markdown(file_path: &str, test_cases: &[TestCase], title: &str) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    
    // Escribir encabezado
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "# Informe de Pruebas: {}", title)?;
    writeln!(file, "\nFecha de ejecución: {}\n", timestamp)?;
    
    // Calcular resumen
    let validated = test_cases.iter().filter(|tc| tc.status == TestStatus::Validated).count();
    let rejected = test_cases.iter().filter(|tc| tc.status == TestStatus::Rejected).count();
    let pending = test_cases.iter().filter(|tc| tc.status == TestStatus::Pending).count();
    let skipped = test_cases.iter().filter(|tc| tc.status == TestStatus::Skipped).count();
    let blocked = test_cases.iter().filter(|tc| tc.status == TestStatus::Blocked).count();
    
    // Escribir resumen al principio
    writeln!(file, "## Resumen\n")?;
    writeln!(file, "- Total de casos: {}", test_cases.len())?;
    writeln!(file, "- ✅ Validados: {}", validated)?;
    writeln!(file, "- ❌ Rechazados: {}", rejected)?;
    writeln!(file, "- ⏳ Pendientes: {}", pending)?;
    writeln!(file, "- ⏭️ Omitidos: {}", skipped)?;
    writeln!(file, "- 🚫 Bloqueados: {}\n", blocked)?;
    
    // Escribir tabla
    writeln!(file, "## Detalle de casos\n")?;
    writeln!(file, "| ID | Descripción | Estado | Observaciones | Evidencia |")?;
    writeln!(file, "|-----|------------|--------|---------------|-----------|")?;
    
    for test_case in test_cases {
        writeln!(
            file,
            "| {} | {} | {} | {} | {} |",
            test_case.id,
            test_case.description,
            test_case.status,
            test_case.observations,
            test_case.evidence
        )?;
    }
    

    
    Ok(())
}

use chrono::Local;
use clap::{Parser, Subcommand};
use colored::*;
use csv::Writer;
use inquire::{Select, Text};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{self, Write};
use std::path::Path;
use uuid::Uuid;

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
    version: String,
    ticket_numbers: String,
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

    // Crear directorios para almacenar pruebas si no existen
    let test_dir = Path::new("tests");
    let definitions_dir = Path::new("definitions");
    let executions_dir = Path::new("executions");

    for dir in &[test_dir, definitions_dir, executions_dir] {
        if !dir.exists() {
            create_dir_all(dir)?;
        }
    }

    match &cli.command {
        Some(Commands::Create { test_type, name }) => create_test_cases(test_type, name.clone())?,
        Some(Commands::Modify { file }) => modify_test_cases(file)?,
        Some(Commands::Execute { file }) => execute_test_cases(file)?,
        Some(Commands::List) => list_test_files()?,
        None => {
            // Menú interactivo si no se proporciona un comando
            let options = vec![
                "Crear casos de prueba",
                "Modificar casos de prueba",
                "Ejecutar casos de prueba",
                "Listar archivos de prueba",
                "Salir",
            ];

            let selection = Select::new("¿Qué deseas hacer?", options).prompt();

            match selection {
                Ok("Crear casos de prueba") => {
                    let test_types = vec!["smoke", "regression", "functional"];
                    let test_type = Select::new("Selecciona el tipo de prueba:", test_types)
                        .prompt()
                        .unwrap_or("smoke");

                    let name = Text::new("Nombre del archivo (opcional):").prompt().ok();

                    create_test_cases(test_type, name)?
                }
                Ok("Modificar casos de prueba") => {
                    let file = select_test_file()?;
                    if let Some(file_path) = file {
                        modify_test_cases(&file_path)?
                    }
                }
                Ok("Ejecutar casos de prueba") => {
                    let file = select_test_file()?;
                    if let Some(file_path) = file {
                        execute_test_cases(&file_path)?
                    }
                }
                Ok("Listar archivos de prueba") => list_test_files()?,
                _ => println!("¡Hasta pronto!"),
            }
        }
    }

    Ok(())
}

/// Selecciona un archivo de prueba existente
fn select_test_file() -> io::Result<Option<String>> {
    // Preguntar si desea seleccionar una definición o una ejecución
    let options = vec!["Definición", "Ejecución"];
    let selection = Select::new("¿Qué tipo de archivo deseas seleccionar?", options).prompt();

    match selection {
        Ok("Definición") => {
            // Obtener archivos de definición
            let definition_files = get_definition_files()?;

            if definition_files.is_empty() {
                println!("{}", "No hay archivos de definición disponibles.".red());
                return Ok(None);
            }

            let selection =
                Select::new("Selecciona un archivo de definición:", definition_files).prompt();

            match selection {
                Ok(file) => Ok(Some(file)),
                Err(_) => Ok(None),
            }
        }
        Ok("Ejecución") => {
            // Obtener archivos de ejecución
            let execution_files = get_execution_files()?;

            if execution_files.is_empty() {
                println!("{}", "No hay archivos de ejecución disponibles.".red());
                return Ok(None);
            }

            let selection =
                Select::new("Selecciona un archivo de ejecución:", execution_files).prompt();

            match selection {
                Ok(file) => Ok(Some(file)),
                Err(_) => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

// Esta función ya no se usa, pero la mantenemos comentada por si se necesita en el futuro
/*
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
*/

/// Lista los archivos de prueba disponibles
fn list_test_files() -> io::Result<()> {
    // Obtener archivos de definición
    let definition_files = get_definition_files()?;

    // Obtener archivos de ejecución
    let execution_files = get_execution_files()?;

    if definition_files.is_empty() && execution_files.is_empty() {
        println!("{}", "No hay archivos de prueba disponibles.".yellow());
        return Ok(());
    }

    // Mostrar archivos de definición
    if !definition_files.is_empty() {
        println!("{}", "Archivos de definición disponibles:".green());
        for (i, file) in definition_files.iter().enumerate() {
            println!("{}: {}", i + 1, file);
        }
        println!();
    } else {
        println!("{}", "No hay archivos de definición disponibles.".yellow());
    }

    // Mostrar archivos de ejecución
    if !execution_files.is_empty() {
        println!("{}", "Archivos de ejecución disponibles:".green());
        for (i, file) in execution_files.iter().enumerate() {
            println!("{}: {}", i + 1, file);
        }
    } else {
        println!("{}", "No hay archivos de ejecución disponibles.".yellow());
    }

    Ok(())
}

/// Crea nuevos casos de prueba
fn create_test_cases(test_type: &str, name: Option<String>) -> io::Result<()> {
    // Validar tipo de prueba
    if test_type != "smoke" && test_type != "regression" && test_type != "functional" {
        println!(
            "{}",
            "Tipo de prueba inválido. Use 'smoke', 'regression' o 'functional'.".red()
        );
        return Ok(());
    }

    // Solicitar versión de prueba
    let version = Text::new("Versión de prueba:")
        .prompt()
        .unwrap_or_else(|_| String::from("1.0.0"));

    // Solicitar números de ticket (opcional)
    let ticket_numbers = Text::new("Número(s) de ticket (opcional):")
        .prompt()
        .unwrap_or_default();

    // Generar nombre de archivo base (sin fecha ni hora)
    let base_name = match name {
        Some(n) if !n.is_empty() => format!("{}-{}", test_type, n),
        _ => format!("{}", test_type),
    };

    // Rutas para archivos base (definiciones)
    let base_csv_path = format!("definitions/{}.csv", base_name);

    // Verificar si ya existe un archivo con ese nombre
    if Path::new(&base_csv_path).exists() {
        let options = vec!["Sí", "No"];
        let selection = Select::new(
            format!(
                "Ya existe un archivo con el nombre '{}'. ¿Deseas sobrescribirlo?",
                base_name
            )
            .as_str(),
            options,
        )
        .prompt();

        if let Ok("No") = selection {
            println!("{}", "Operación cancelada.".yellow());
            return Ok(());
        }
    }

    // Crear casos de prueba
    let mut test_cases = Vec::new();
    let mut i = 1;

    println!(
        "{}",
        "Ingresa los casos de prueba. Escribe 'FIN' en la descripción para terminar.".blue()
    );

    loop {
        println!("{}", format!("Caso de prueba #{}", i).blue());

        let id = Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("TC")
            .to_string();

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
            version: version.clone(),
            ticket_numbers: ticket_numbers.clone(),
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
    let selection = Select::new(
        "¿Deseas corregir la ortografía de las descripciones?",
        options,
    )
    .prompt();

    let final_test_cases = if let Ok("Sí") = selection {
        // Verificar si la variable de entorno GROQ_API_KEY está configurada
        if std::env::var("GROQ_API_KEY").is_err() {
            println!(
                "{}",
                "ADVERTENCIA: No se encontró la clave API de Groq.".yellow()
            );
            println!("{}", "Para usar la corrección ortográfica, configura la variable de entorno GROQ_API_KEY.".yellow());
            println!("{}", "Ejemplo: export GROQ_API_KEY=tu-clave-api".yellow());

            // Preguntar si desea continuar sin corrección ortográfica
            let continue_options = vec!["Continuar sin corrección", "Cancelar"];
            let continue_selection = Select::new("¿Qué deseas hacer?", continue_options).prompt();

            if let Ok("Cancelar") = continue_selection {
                println!("{}", "Operación cancelada.".yellow());
                return Ok(());
            }

            // Continuar sin corrección ortográfica
            println!("{}", "Continuando sin corrección ortográfica.".blue());
            // Usar los casos de prueba sin corrección
            test_cases.clone()
        } else {
            println!(
                "{}",
                "Corrigiendo ortografía usando la API de Groq...".blue()
            );

            // Corregir ortografía de las descripciones
            let mut corrected_cases = Vec::new();

            for mut test_case in test_cases {
                let corrected_description = correct_spelling(&test_case.description);

                // Solo actualizar si hay cambios
                if corrected_description != test_case.description {
                    println!(
                        "{}",
                        format!(
                            "Corrección: '{}' -> '{}'",
                            test_case.description, corrected_description
                        )
                        .green()
                    );
                    test_case.description = corrected_description;
                }

                corrected_cases.push(test_case);
            }

            corrected_cases
        }
    } else {
        test_cases
    };

    // Guardar el archivo base (definición) en CSV
    save_to_csv(&base_csv_path, &final_test_cases)?;

    // Preguntar si desea ejecutar los casos de prueba ahora
    let options = vec!["Sí", "No"];
    let selection = Select::new("¿Deseas ejecutar estos casos de prueba ahora?", options).prompt();

    if let Ok("Sí") = selection {
        // Ejecutar los casos de prueba recién creados
        execute_test_cases_from_definition(&base_csv_path)?;
    } else {
        println!(
            "{}",
            format!(
                "Definición de casos de prueba creada y guardada en {}",
                base_csv_path
            )
            .green()
        );
        println!("{}", "Puedes ejecutar estos casos de prueba más tarde seleccionando 'Ejecutar casos de prueba' en el menú principal.".blue());
    }

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
        println!(
            "{}: {} - {} - {}",
            i + 1,
            test_case.id,
            test_case.description,
            test_case.status
        );
    }

    // Seleccionar caso de prueba a modificar
    let selection = Select::new(
        "Selecciona un caso de prueba para modificar:",
        (0..test_cases.len())
            .map(|i| format!("{}: {}", i + 1, test_cases[i].description))
            .collect::<Vec<_>>(),
    )
    .prompt();

    match selection {
        Ok(selected) => {
            let index = selected
                .split(':')
                .next()
                .unwrap_or("1")
                .parse::<usize>()
                .unwrap_or(1)
                - 1;

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
                    }
                    "Estado" => {
                        let status_options = vec![
                            "⏳ Pendiente",
                            "✅ Validado",
                            "❌ Rechazado",
                            "⏭️ Omitido",
                            "🚫 Bloqueado",
                        ];

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
                    }
                    "Observaciones" => {
                        let new_observations = Text::new("Nuevas observaciones:")
                            .with_initial_value(&test_cases[index].observations)
                            .prompt()
                            .unwrap_or_else(|_| test_cases[index].observations.clone());

                        test_cases[index].observations = new_observations;
                    }
                    "Evidencia" => {
                        let new_evidence = Text::new("Nueva evidencia (ruta o URL):")
                            .with_initial_value(&test_cases[index].evidence)
                            .prompt()
                            .unwrap_or_else(|_| test_cases[index].evidence.clone());

                        test_cases[index].evidence = new_evidence;
                    }
                    _ => {}
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
        }
        Err(_) => println!("{}", "Operación cancelada.".yellow()),
    }

    Ok(())
}

/// Ejecuta casos de prueba a partir de un archivo de definición
fn execute_test_cases_from_definition(definition_path: &str) -> io::Result<()> {
    // Cargar los casos de prueba desde el archivo de definición
    let mut test_cases = load_from_csv(definition_path)?;

    if test_cases.is_empty() {
        println!("{}", "No hay casos de prueba para ejecutar.".yellow());
        return Ok(());
    }

    // Generar nombre para el archivo de ejecución
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");

    // Obtener el nombre base del archivo de definición
    let base_name = Path::new(definition_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("test_cases");

    // Crear rutas para los archivos de ejecución
    let execution_name = format!("{}-{}", base_name, timestamp);
    let execution_csv_path = format!("executions/{}.csv", execution_name);
    let execution_md_path = format!("executions/{}.md", execution_name);

    println!(
        "{}",
        format!(
            "Ejecutando casos de prueba de la definición {}",
            definition_path
        )
        .blue()
    );
    println!(
        "{}",
        format!("Los resultados se guardarán en {}", execution_csv_path).blue()
    );

    // Ejecutar los casos de prueba
    for i in 0..test_cases.len() {
        println!(
            "{}",
            format!("Caso de prueba #{}: {}", i + 1, test_cases[i].description.clone()).blue()
        );

        // Mostrar estado actual
        println!("Estado actual: {}", test_cases[i].status);

        // Seleccionar nuevo estado
        let status_options = vec![
            "⏳ Pendiente",
            "✅ Validado",
            "❌ Rechazado",
            "⏭️ Omitido",
            "🚫 Bloqueado",
        ];

        let new_status = Select::new("Selecciona el resultado de la ejecución:", status_options)
            .prompt()
            .unwrap_or("⏳ Pendiente");

        test_cases[i].status = match new_status {
            "✅ Validado" => TestStatus::Validated,
            "❌ Rechazado" => TestStatus::Rejected,
            "⏭️ Omitido" => TestStatus::Skipped,
            "🚫 Bloqueado" => TestStatus::Blocked,
            _ => TestStatus::Pending,
        };

        // Agregar observaciones
        let current_observations = test_cases[i].observations.clone();
        let observations = Text::new("Observaciones (opcional):")
            .with_initial_value(&current_observations)
            .prompt()
            .unwrap_or_else(|_| current_observations);

        test_cases[i].observations = observations;

        // Agregar evidencia
        let current_evidence = test_cases[i].evidence.clone();
        let evidence = Text::new("Evidencia (ruta o URL, opcional):")
            .with_initial_value(&current_evidence)
            .prompt()
            .unwrap_or_else(|_| current_evidence);

        test_cases[i].evidence = evidence;
    }

    // Guardar resultados en los archivos de ejecución
    save_to_csv(&execution_csv_path, &test_cases)?;
    save_to_markdown(&execution_md_path, &test_cases, &execution_name)?;

    println!(
        "{}",
        format!(
            "Ejecución de casos de prueba completada y guardada en {} y {}",
            execution_csv_path, execution_md_path
        )
        .green()
    );

    Ok(())
}

/// Ejecuta casos de prueba
fn execute_test_cases(file_path: &str) -> io::Result<()> {
    // Verificar si el archivo es una definición o una ejecución anterior
    if file_path.starts_with("definitions/") {
        // Si es una definición, ejecutar a partir de ella
        execute_test_cases_from_definition(file_path)
    } else {
        // Si es una ejecución anterior, mostrar mensaje y preguntar
        println!("{}", "NOTA: Estás ejecutando a partir de un archivo de ejecución anterior, no de una definición base.".yellow());

        let options = vec![
            "Continuar con este archivo",
            "Seleccionar una definición base",
        ];
        let selection = Select::new("¿Qué deseas hacer?", options).prompt();

        match selection {
            Ok("Seleccionar una definición base") => {
                // Listar archivos de definición
                let definitions = get_definition_files()?;

                if definitions.is_empty() {
                    println!("{}", "No hay archivos de definición disponibles.".yellow());
                    return Ok(());
                }

                let selection =
                    Select::new("Selecciona un archivo de definición:", definitions).prompt();

                match selection {
                    Ok(definition_path) => execute_test_cases_from_definition(&definition_path),
                    Err(_) => {
                        println!("{}", "Operación cancelada.".yellow());
                        Ok(())
                    }
                }
            }
            _ => {
                // Ejecutar a partir del archivo seleccionado (ejecución anterior)
                let mut test_cases = load_from_csv(file_path)?;

                if test_cases.is_empty() {
                    println!("{}", "No hay casos de prueba para ejecutar.".yellow());
                    return Ok(());
                }

                // Generar nombre para el nuevo archivo de ejecución
                let timestamp = Local::now().format("%Y%m%d_%H%M%S");

                // Obtener el nombre base del archivo anterior
                let file_name = Path::new(file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("test_cases");

                // Eliminar el timestamp anterior si existe
                let base_name = if file_name.contains('-') {
                    file_name
                        .split('-')
                        .take(file_name.split('-').count() - 1)
                        .collect::<Vec<&str>>()
                        .join("-")
                } else {
                    file_name.to_string()
                };

                // Crear rutas para los nuevos archivos de ejecución
                let execution_name = format!("{}-{}", base_name, timestamp);
                let execution_csv_path = format!("executions/{}.csv", execution_name);
                let execution_md_path = format!("executions/{}.md", execution_name);

                println!(
                    "{}",
                    format!("Ejecutando casos de prueba a partir de {}", file_path).blue()
                );
                println!(
                    "{}",
                    format!("Los resultados se guardarán en {}", execution_csv_path).blue()
                );

                // Ejecutar los casos de prueba
                for i in 0..test_cases.len() {
                    println!(
                        "{}",
                        format!("Caso de prueba #{}: {}", i + 1, test_cases[i].description.clone()).blue()
                    );

                    // Mostrar estado actual
                    println!("Estado actual: {}", test_cases[i].status);

                    // Seleccionar nuevo estado
                    let status_options = vec![
                        "⏳ Pendiente",
                        "✅ Validado",
                        "❌ Rechazado",
                        "⏭️ Omitido",
                        "🚫 Bloqueado",
                    ];

                    let new_status =
                        Select::new("Selecciona el resultado de la ejecución:", status_options)
                            .prompt()
                            .unwrap_or("⏳ Pendiente");

                    test_cases[i].status = match new_status {
                        "✅ Validado" => TestStatus::Validated,
                        "❌ Rechazado" => TestStatus::Rejected,
                        "⏭️ Omitido" => TestStatus::Skipped,
                        "🚫 Bloqueado" => TestStatus::Blocked,
                        _ => TestStatus::Pending,
                    };

                    // Agregar observaciones
                    let current_observations = test_cases[i].observations.clone();
                    let observations = Text::new("Observaciones (opcional):")
                        .with_initial_value(&current_observations)
                        .prompt()
                        .unwrap_or_else(|_| current_observations);

                    test_cases[i].observations = observations;

                    // Agregar evidencia
                    let current_evidence = test_cases[i].evidence.clone();
                    let evidence = Text::new("Evidencia (ruta o URL, opcional):")
                        .with_initial_value(&current_evidence)
                        .prompt()
                        .unwrap_or_else(|_| current_evidence);

                    test_cases[i].evidence = evidence;
                }

                // Guardar resultados en los nuevos archivos de ejecución
                save_to_csv(&execution_csv_path, &test_cases)?;
                save_to_markdown(&execution_md_path, &test_cases, &execution_name)?;

                println!(
                    "{}",
                    format!(
                        "Ejecución de casos de prueba completada y guardada en {} y {}",
                        execution_csv_path, execution_md_path
                    )
                    .green()
                );

                Ok(())
            }
        }
    }
}

/// Obtiene la lista de archivos de definición disponibles
fn get_definition_files() -> io::Result<Vec<String>> {
    let mut definition_files = Vec::new();

    // Verificar si el directorio de definiciones existe
    let definitions_dir = Path::new("definitions");
    if !definitions_dir.exists() {
        create_dir_all(definitions_dir)?;
        return Ok(definition_files);
    }

    // Leer los archivos del directorio
    for entry in read_dir(definitions_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Solo incluir archivos CSV
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("csv") {
            if let Some(path_str) = path.to_str() {
                definition_files.push(path_str.to_string());
            }
        }
    }

    // Ordenar por nombre
    definition_files.sort();

    Ok(definition_files)
}

/// Obtiene la lista de archivos de ejecución disponibles
fn get_execution_files() -> io::Result<Vec<String>> {
    let mut execution_files = Vec::new();

    // Verificar si el directorio de ejecuciones existe
    let executions_dir = Path::new("executions");
    if !executions_dir.exists() {
        create_dir_all(executions_dir)?;
    } else {
        // Leer los archivos del directorio
        for entry in read_dir(executions_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Solo incluir archivos CSV
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("csv") {
                if let Some(path_str) = path.to_str() {
                    execution_files.push(path_str.to_string());
                }
            }
        }
    }

    // Verificar también en el directorio tests (para compatibilidad con versiones anteriores)
    let tests_dir = Path::new("tests");
    if tests_dir.exists() {
        for entry in read_dir(tests_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("csv") {
                if let Some(path_str) = path.to_str() {
                    execution_files.push(path_str.to_string());
                }
            }
        }
    }

    // Ordenar por fecha (más recientes primero)
    execution_files.sort_by(|a, b| b.cmp(a));

    Ok(execution_files)
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

/// Corrige la ortografía de un texto utilizando la API de Groq
fn correct_spelling(text: &str) -> String {
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
                "content": format!("Corrige los errores ortográficos y gramaticales en el siguiente texto, manteniendo su significado original: {}", text)
            }
        ],
        "model": "llama3-8b-8192"
    });

    // Intentar hacer la solicitud a la API de Groq
    match client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
    {
        Ok(response) => {
            // Verificar si la respuesta es exitosa
            if response.status().is_success() {
                // Intentar parsear la respuesta JSON
                match response.json::<Value>() {
                    Ok(json) => {
                        // Obtener el contenido corregido
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            if let Some(first_choice) = choices.first() {
                                if let Some(message) = first_choice.get("message") {
                                    if let Some(content) =
                                        message.get("content").and_then(|c| c.as_str())
                                    {
                                        // Devolver el texto corregido
                                        return content.trim().to_string();
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            format!("Error al parsear la respuesta JSON: {}", e).red()
                        );
                    }
                }
            } else {
                // Guardar el estado antes de consumir response
                let status = response.status();

                // Intentar obtener el mensaje de error
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

fn save_to_markdown(file_path: &str, test_cases: &[TestCase], title: &str) -> io::Result<()> {
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
        .filter(|tc| tc.status == TestStatus::Validated)
        .count();
    let rejected = test_cases
        .iter()
        .filter(|tc| tc.status == TestStatus::Rejected)
        .count();
    let pending = test_cases
        .iter()
        .filter(|tc| tc.status == TestStatus::Pending)
        .count();
    let skipped = test_cases
        .iter()
        .filter(|tc| tc.status == TestStatus::Skipped)
        .count();
    let blocked = test_cases
        .iter()
        .filter(|tc| tc.status == TestStatus::Blocked)
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
        writeln!(file, "    \"⏭️ Omitidos\" : {}", skipped)?; // Gris
    }
    if blocked > 0 {
        writeln!(file, "    \"🚫 Bloqueados\" : {}", blocked)?; // Naranja
    }

    writeln!(file, "```\n")?;

    // Escribir tabla
    writeln!(file, "## Detalle de casos\n")?;
    writeln!(
        file,
        "| ID | Descripción | Estado | Observaciones | Evidencia |"
    )?;
    writeln!(
        file,
        "|-----|------------|--------|---------------|-----------|"
    )?;

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

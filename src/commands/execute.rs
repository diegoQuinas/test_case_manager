use std::io;
use std::path::Path;
use chrono::Local;
use colored::*;
use inquire::{Select, Text};

use crate::models::{TestCase, TestStatus};
use crate::utils::{get_definition_files, load_from_csv, save_to_csv, save_to_markdown};

/// Ejecuta casos de prueba
pub fn execute_test_cases(file_path: &str) -> io::Result<()> {
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

                // Obtener el nombre base del archivo de ejecución anterior
                let base_name = Path::new(file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("test_cases");

                // Extraer el nombre base sin el timestamp anterior
                let name_parts: Vec<&str> = base_name.split('-').collect();
                let clean_base_name = if name_parts.len() > 1 {
                    name_parts[0..name_parts.len() - 1].join("-")
                } else {
                    base_name.to_string()
                };

                // Crear rutas para los nuevos archivos de ejecución
                let execution_name = format!("{}-{}", clean_base_name, timestamp);
                let execution_csv_path = format!("executions/{}.csv", execution_name);
                let execution_md_path = format!("executions/{}.md", execution_name);

                println!(
                    "{}",
                    format!(
                        "Continuando ejecución a partir de {}",
                        file_path
                    )
                    .blue()
                );
                println!(
                    "{}",
                    format!("Los resultados se guardarán en {}", execution_csv_path).blue()
                );

                // Ejecutar los casos de prueba
                execute_test_cases_impl(&mut test_cases, &execution_csv_path, &execution_md_path, &execution_name)
            }
        }
    }
}

/// Ejecuta casos de prueba a partir de un archivo de definición
pub fn execute_test_cases_from_definition(definition_path: &str) -> io::Result<()> {
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
    execute_test_cases_impl(&mut test_cases, &execution_csv_path, &execution_md_path, &execution_name)
}

/// Implementación de la ejecución de casos de prueba
fn execute_test_cases_impl(
    test_cases: &mut [TestCase],
    execution_csv_path: &str,
    execution_md_path: &str,
    execution_name: &str,
) -> io::Result<()> {
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
    save_to_csv(execution_csv_path, test_cases)?;
    save_to_markdown(execution_md_path, test_cases, execution_name)?;

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

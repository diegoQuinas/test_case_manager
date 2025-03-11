use std::io;
use std::path::Path;
use colored::*;
use inquire::{Select, Text};

use crate::models::{TestCase, TestStatus};
use crate::utils::{load_from_csv, save_to_csv, save_to_markdown};

/// Modifica casos de prueba existentes
pub fn modify_test_cases(file_path: &str) -> io::Result<()> {
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
                let options = vec!["Descripción", "Estado", "Observaciones", "Evidencia", "Versión", "Ticket(s)"];

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
                    "Versión" => {
                        let new_version = Text::new("Nueva versión:")
                            .with_initial_value(&test_cases[index].version)
                            .prompt()
                            .unwrap_or_else(|_| test_cases[index].version.clone());

                        test_cases[index].version = new_version;
                    }
                    "Ticket(s)" => {
                        let new_tickets = Text::new("Nuevos ticket(s):")
                            .with_initial_value(&test_cases[index].ticket_numbers)
                            .prompt()
                            .unwrap_or_else(|_| test_cases[index].ticket_numbers.clone());

                        test_cases[index].ticket_numbers = new_tickets;
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

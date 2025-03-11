use std::io;
use std::path::Path;
use colored::*;
use inquire::{Select, Text};
use uuid::Uuid;

use crate::models::{TestCase, TestStatus};
use crate::utils::{correct_spelling, save_to_csv, save_to_markdown};
use crate::commands::execute::execute_test_cases_from_definition;

/// Crea nuevos casos de prueba
pub fn create_test_cases(test_type: &str, name: Option<String>) -> io::Result<()> {
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

    // Aplicar corrección ortográfica si se seleccionó "Sí"
    let final_test_cases = if let Ok("Sí") = selection {
        println!("{}", "Corrigiendo ortografía...".blue());

        let mut corrected_cases = Vec::new();

        for mut test_case in test_cases {
            println!(
                "{}",
                format!("Corrigiendo: {}", test_case.description).blue()
            );

            // Corregir ortografía
            let corrected_description = correct_spelling(&test_case.description);

            // Verificar si hubo cambios
            if corrected_description != test_case.description {
                println!(
                    "{}",
                    format!(
                        "Descripción corregida: {} -> {}",
                        test_case.description, corrected_description
                    )
                    .green()
                );
                test_case.description = corrected_description;
            }

            corrected_cases.push(test_case);
        }

        corrected_cases
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

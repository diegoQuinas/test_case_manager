# CLI Test Case Manager

Una herramienta de línea de comandos para gestionar casos de prueba para pruebas de regresión y pruebas de humo.

## Características

- Crear casos de prueba para pruebas de regresión o pruebas de humo
- Guardar casos de prueba en formato CSV y Markdown
- Modificar casos de prueba existentes
- Ejecutar casos de prueba y actualizar su estado
- Interfaz interactiva con emojis para indicar el estado de las pruebas
- Generar informes con fecha y hora de ejecución

## Estados de Prueba

- ⏳ Pendiente
- ✅ Validado
- ❌ Rechazado
- ⏭️ Omitido
- 🚫 Bloqueado

## Instalación

```bash
cargo build --release
```

El ejecutable estará disponible en `target/release/test_case_manager`.

## Uso

### Modo Interactivo

Simplemente ejecuta el programa sin argumentos:

```bash
./test_case_manager
```

### Comandos Específicos

#### Crear casos de prueba

```bash
./test_case_manager create --test-type smoke --name login
./test_case_manager create --test-type regression
```

#### Modificar casos de prueba

```bash
./test_case_manager modify --file tests/smoke-login-20250311_112345.csv
```

#### Ejecutar casos de prueba

```bash
./test_case_manager execute --file tests/smoke-login-20250311_112345.csv
```

#### Listar archivos de prueba

```bash
./test_case_manager list
```

## Estructura de Archivos

Los casos de prueba se guardan en la carpeta `tests/` con los siguientes formatos:

- CSV: `tests/{tipo}-{nombre}-{timestamp}.csv`
- Markdown: `tests/{tipo}-{nombre}-{timestamp}.md`

## Ejemplo de Tabla Markdown

| ID | Descripción | Estado | Observaciones | Evidencia |
|-----|------------|--------|---------------|-----------|
| abc123 | Verificar login | ✅ Validado | Funciona correctamente | screenshot.png |
| def456 | Verificar registro | ❌ Rechazado | Error en validación | error_log.txt |

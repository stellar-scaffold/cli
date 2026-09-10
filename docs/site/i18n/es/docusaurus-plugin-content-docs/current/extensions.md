# Extensiones

Las extensiones de Scaffold te permiten conectarte al ciclo de vida de la compilación sin modificar Scaffold en sí. Una extensión es un binario común en tu `PATH` llamado `stellar-scaffold-<name>`. Scaffold la descubre automáticamente, le pregunta qué hooks le interesan, y la invoca en cada uno de esos puntos durante un ciclo de compilación o watch.

Las extensiones pueden hacer cualquier cosa: registrar métricas, ejecutar auditorías, publicar notificaciones en Slack, escribir artefactos personalizados, aplicar límites de tamaño, o generar archivos adicionales. Reciben contexto detallado sobre lo que acaba de suceder y pueden escribir lo que quieran en stdout, que Scaffold reenvía a la terminal del usuario.

---

## Ciclo de vida de los hooks

Cada compilación pasa por la misma secuencia ordenada de hooks:

```
pre-dev
  └─ pre-compile
       └─ [cargo build por contrato]
  └─ post-compile
  └─ pre-deploy   (por contrato)
       └─ [subir wasm, desplegar/actualizar contrato]
  └─ post-deploy  (por contrato)
  └─ pre-codegen  (por contrato)
       └─ [stellar contract bindings typescript + npm build]
  └─ post-codegen (por contrato)
post-dev
```

Los hooks `pre-compile` y `post-compile` se disparan una vez por ciclo de compilación, cubriendo todos los contratos. Los hooks `pre-deploy`, `post-deploy`, `pre-codegen` y `post-codegen` se disparan una vez **por contrato**. Los hooks `pre-dev` y `post-dev` delimitan todo el ciclo.

Solo necesitas manejar los hooks relevantes para tu extensión. Los hooks que no listes en tu manifiesto nunca se invocan.

---

## Registrar una extensión

Agrega tu extensión a `environments.toml` bajo los entornos en los que debe ejecutarse:

```toml
[development]
extensions = ["reporter"]

[staging]
extensions = ["reporter", "audit-tool"]
```

### Configuración por extensión

Puedes pasar configuración arbitraria a una extensión mediante `[<env>.ext.<name>]`:

```toml
[development.ext.reporter]
warn_size_kb = 128
log_file = ".scaffold/reports/dev.log"
```

Scaffold serializa esta tabla y la inyecta como el campo `config` en cada invocación de hook para esa extensión. Si no existe una sección de configuración, `config` está ausente del JSON.

---

## Cómo Scaffold invoca una extensión

1. **Descubrimiento:** Al iniciar, Scaffold ejecuta `stellar-scaffold-<name> manifest` y analiza la respuesta JSON para saber qué hooks quiere la extensión.
2. **Invocación:** En cada punto del ciclo de vida para el que la extensión se registró, Scaffold ejecuta `stellar-scaffold-<name> <hook-name>` y escribe un objeto JSON en su stdin.
3. **Salida:** La extensión lee stdin, hace su trabajo, escribe lo que quiera en stdout (que se reenvía a la terminal del usuario), y termina. Un código de salida distinto de cero se registra como error, pero Scaffold continúa — las demás extensiones registradas para el mismo hook siguen ejecutándose y la compilación no se aborta.

---

## El subcomando `manifest`

Tu binario debe responder a `manifest` escribiendo un objeto JSON en stdout:

```json
{
  "name": "my-extension",
  "version": "1.0.0",
  "hooks": ["post-compile", "post-deploy"]
}
```

Lista solo los hooks que tu extensión realmente maneja. Listar un hook que no manejas desperdicia una invocación de subproceso en cada compilación. El `name` debe coincidir con el sufijo de tu binario (`stellar-scaffold-my-extension` → `"my-extension"`).

---

## El JSON de stdin

En cada invocación de hook, Scaffold escribe un objeto JSON plano en el stdin de la extensión. El objeto siempre incluye `config` (la configuración de tu extensión desde `environments.toml`, o `null` si no se proporcionó ninguna) más campos de contexto que dependen de qué hook se está disparando.

### Referencia de campos

| Campo | pre/post-compile | pre/post-deploy | pre/post-codegen | pre/post-dev |
| --- | --- | --- | --- | --- |
| `config` | ✓ | ✓ | ✓ | ✓ |
| `project_root` | ✓ | ✓ | ✓ | ✓ |
| `env` | ✓ | ✓ | ✓ | ✓ |
| `wasm_out_dir` | ✓ | ✓ | ✓ | ✓ |
| `source_dirs` | ✓ | ✓ | ✓ | ✓ |
| `wasm_paths` | ✓ (vacío en pre) | ✓ | ✓ | — |
| `network` | — | ✓ | ✓ | ✓ (si `--build-clients`) |
| `contract_name` | — | ✓ | ✓ | — |
| `wasm_path` | — | ✓ | ✓ | — |
| `wasm_hash` | — | ✓ | ✓ | — |
| `contract_id` | — | ✓ (`null` en pre) | ✓ | — |
| `ts_package_dir` | — | — | ✓ | — |
| `src_template_path` | — | — | ✓ | — |
| `contracts` | — | — | — | ✓ |
| `watch_paths` | — | — | — | ✓ |

### Descripción de los campos

| Campo | Tipo | Descripción |
| --- | --- | --- |
| `config` | object \| null | La tabla de configuración de tu extensión desde `environments.toml`, o `null` |
| `project_root` | string (ruta) | Ruta absoluta a la raíz del workspace de Cargo |
| `env` | string | Entorno activo: `"development"`, `"testing"`, `"staging"`, o `"production"` |
| `wasm_out_dir` | string (ruta) | Directorio donde se escriben los archivos WASM compilados |
| `source_dirs` | string[] | Directorios de origen de los contratos en orden topológico de compilación |
| `wasm_paths` | object | Mapa de `contract_name → wasm_path`; vacío en `pre-compile` |
| `network` | object \| null | URL de RPC resuelta, passphrase de red y nombre de red |
| `contract_name` | string | Nombre de contrato en snake_case que coincide con el nombre base del archivo WASM |
| `wasm_path` | string (ruta) | Ruta absoluta al WASM compilado de este contrato |
| `wasm_hash` | string | SHA-256 codificado en hex del bytecode WASM subido |
| `contract_id` | string \| null | Dirección de contrato de Stellar (strkey `C…`); `null` en `pre-deploy` |
| `ts_package_dir` | string (ruta) | El paquete de cliente generado de este contrato — `<clients_dir>/<name>/`, es decir `<project_root>/app-lib/clients/<name>/` por defecto |
| `src_template_path` | string (ruta) | El archivo índice que comparten todos los clientes generados — `<clients_dir>/index.ts`, es decir `<project_root>/app-lib/clients/index.ts` por defecto |
| `contracts` | object[] | Arreglo de resumen por contrato; los campos opcionales son `null` en `pre-dev` |
| `watch_paths` | string[] | Directorios que se están observando; vacío en compilaciones de una sola vez |

### Ejemplo: stdin de `post-compile`

```json
{
  "config": null,
  "project_root": "/path/to/my-project",
  "env": "development",
  "wasm_out_dir": "/path/to/my-project/target/stellar/local",
  "source_dirs": ["/path/to/my-project/contracts/hello_world"],
  "wasm_paths": {
    "hello_world": "/path/to/my-project/target/stellar/local/hello_world.wasm"
  }
}
```

### Ejemplo: stdin de `post-deploy`

```json
{
  "config": { "warn_size_kb": 128 },
  "project_root": "/path/to/my-project",
  "env": "development",
  "wasm_out_dir": "/path/to/my-project/target/stellar/local",
  "source_dirs": ["/path/to/my-project/contracts/hello_world"],
  "wasm_paths": {
    "hello_world": "/path/to/my-project/target/stellar/local/hello_world.wasm"
  },
  "network": {
    "rpc_url": "http://localhost:8000/soroban/rpc",
    "network_passphrase": "Standalone Network ; February 2017",
    "network_name": "local"
  },
  "contract_name": "hello_world",
  "wasm_path": "/path/to/my-project/target/stellar/local/hello_world.wasm",
  "wasm_hash": "a1b2c3d4e5f6...",
  "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"
}
```

---

## Construir una extensión

### Paso 1: Crear un crate binario

```sh
cargo new --bin stellar-scaffold-my-extension
cd stellar-scaffold-my-extension
```

### Paso 2: Implementar el subcomando `manifest`

Tu binario debe manejar `manifest` como su primer argumento e imprimir JSON en stdout:

```sh
stellar-scaffold-my-extension manifest
```

```json
{
  "name": "my-extension",
  "version": "1.0.0",
  "hooks": ["post-compile", "post-deploy"]
}
```

Lista solo los hooks que realmente manejas.

### Paso 3: Implementar los manejadores de hooks

Para cada hook que listaste, maneja el argumento de subcomando correspondiente. Lee el JSON completo de stdin, haz tu trabajo, e imprime salida para el usuario:

```sh
stellar-scaffold-my-extension post-compile
# (JSON en stdin)
```

### Paso 4: Instalarlo en el PATH

Scaffold descubre extensiones buscando binarios llamados `stellar-scaffold-<name>` en tu `PATH`. Para extensiones en Rust, instala con Cargo:

```sh
cargo install --path .
```

O copia el binario compilado a algún lugar de tu `PATH`.

### Paso 5: Registrarla en `environments.toml`

```toml
[development]
extensions = ["my-extension"]
```

Ejecuta `stellar scaffold build` o `stellar scaffold watch` y tu extensión será invocada en cada hook registrado.

---

## Ejemplos específicos por lenguaje

### Rust

Usa el crate [`stellar-scaffold-ext-types`](https://crates.io/crates/stellar-scaffold-ext-types) para acceso tipado al JSON de stdin:

```toml
[dependencies]
stellar-scaffold-ext-types = "0.0.1"
serde_json = "1"
```

```rust
use std::io::Read;
use stellar_scaffold_ext_types::{ExtensionManifest, HookName, CompileContext};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("manifest") => {
            let manifest = ExtensionManifest {
                name: "my-extension".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                hooks: vec![HookName::PostCompile.as_str().to_string()],
            };
            println!("{}", serde_json::to_string(&manifest).unwrap());
        }
        Some("post-compile") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap();
            let ctx: CompileContext = serde_json::from_str(&buf).unwrap();
            println!("Compiled {} contracts:", ctx.wasm_paths.len());
            for (name, path) in &ctx.wasm_paths {
                println!("  {name}: {}", path.display());
            }
        }
        _ => {}
    }
}
```

El crate `ext-types` usa `#[serde(flatten)]` para que los structs de Rust se compongan de forma natural mientras el formato en el cable se mantiene plano. Consulta el [README del crate](https://github.com/stellar-scaffold/cli/tree/main/crates/stellar-scaffold-ext-types) para la referencia completa de tipos.

### TypeScript / Node.js

```ts
import * as readline from "readline";

const args = process.argv.slice(2);

if (args[0] === "manifest") {
  console.log(
    JSON.stringify({
      name: "my-extension",
      version: "1.0.0",
      hooks: ["post-compile"],
    }),
  );
} else if (args[0] === "post-compile") {
  let input = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => (input += chunk));
  process.stdin.on("end", () => {
    const ctx = JSON.parse(input);
    const names = Object.keys(ctx.wasm_paths);
    console.log(`Compiled ${names.length} contracts: ${names.join(", ")}`);
  });
}
```

Instálalo colocando el script en tu `PATH` (mediante un shebang + `chmod +x`, un bundle compilado con `pkg` o `bun build --compile`, etc.) y nombrándolo `stellar-scaffold-my-extension`.

### Cualquier otro lenguaje

Las extensiones son solo binarios. Usa el lenguaje que quieras. Los únicos requisitos son:

- El binario se llama `stellar-scaffold-<name>` y está en tu `PATH`
- Ejecutarlo con `manifest` imprime un manifiesto JSON en stdout
- Ejecutarlo con un nombre de hook lee JSON desde stdin y termina con código 0 en caso de éxito

---

## El Scaffold Reporter

`stellar-scaffold-reporter` es la implementación de referencia canónica. Es la extensión integrada que viene con cada proyecto creado con `stellar scaffold init` y demuestra el ciclo de vida completo de los hooks en la práctica.

Rastrea y registra:

- **Tiempo de compilación** — cuánto tardó `cargo build`
- **Tamaños de WASM** — tamaño en bytes de la salida `.wasm` de cada contrato, con la diferencia respecto a la compilación anterior
- **Información de despliegue** — ID de contrato, hash de WASM y duración del despliegue por contrato
- **Tamaño del paquete de TypeScript** — tamaño total del paquete de cliente generado
- **Duración total del ciclo de compilación** — tiempo de extremo a extremo desde `pre-dev` hasta `post-dev`

Puedes instalarlo de forma independiente con:

```sh
cargo install stellar-scaffold-reporter
```

Y registrarlo en `environments.toml`:

```toml
[development]
extensions = ["reporter"]
```

Explora el [código fuente](https://github.com/stellar-scaffold/cli/tree/main/crates/stellar-scaffold-reporter) y su [README](https://github.com/stellar-scaffold/cli/tree/main/crates/stellar-scaffold-reporter/README.md) para ver una extensión completa y real que maneja los ocho hooks.

# Comandos de la CLI

Stellar Scaffold proporciona varios comandos CLI para ayudarte a gestionar el desarrollo de tus contratos inteligentes en Stellar.

## Comando Init

Inicializa un nuevo proyecto de Stellar Scaffold:

```bash
stellar scaffold init <project-path>
```

Opciones:

- `project-path`: Requerido. La ruta donde se creará el proyecto
- `--template <name>`: Selector de plantilla. Un nombre de framework simple (`react` o `svelte`) selecciona una plantilla oficial; una abreviatura `user/repo` (opcionalmente con `#branch` o `#tag`) descarga ese repo comunitario directamente con degit; `none` crea un proyecto solo de contratos sin frontend. Omítelo para elegir de forma interactiva
- `--no-template`: Crea un proyecto solo de contratos sin frontend (alias de `--template none`)
- `--tutorial`: Comienza desde el proyecto simplificado sobre el que se construye el [tutorial](./tutorial/00-overview.md), en lugar de una plantilla completa
- `-p <name>` o `--package-manager <name>`: Gestor de paquetes a usar. Omítelo para elegir de forma interactiva
- `-y` o `--yes`: Acepta todos los valores por defecto y omite las preguntas interactivas

`--tutorial` selecciona el frontend y el gestor de paquetes por ti, así que no pregunta. Pasa `--package-manager` junto con él si quieres algo distinto de npm.

El comando init crea:

- Un nuevo proyecto de contrato inteligente de Stellar con buenas prácticas y configuraciones
- Una aplicación frontend en `app/`, en el framework que elegiste
- `app-lib/`, código de utilidad que tu app puede usar para la conexión de wallet, configuración de red y formato, además del directorio donde se escriben tus clientes de contrato generados
- Archivos de configuración tanto para el desarrollo de contratos como del frontend

Las plantillas oficiales provienen del [repo de plantillas de Stellar Scaffold](https://github.com/stellar-scaffold/ui).

Con `--no-template` (o `--template none`), la capa de frontend se omite por completo: sin `app/`, sin workspaces de JS, sin instalación de dependencias. Cada contrato en `environments.toml` se escribe con `client = false`, de modo que `stellar scaffold build` y `watch` omiten la generación de paquetes de cliente. Vuelve a poner `client = true` en un contrato si más adelante necesitas clientes de TypeScript.

## Comando Generate

Genera un nuevo contrato a partir de ejemplos o del asistente. Consulta su [documentación](https://docs.openzeppelin.com/stellar-contracts) oficial.

```bash
stellar scaffold generate contract [options]
```

Opciones:

- `--from <example>`: Clona un contrato de uno de dos conjuntos de ejemplos, seleccionado por prefijo:
  - `oz/<name>` — un ejemplo de [OpenZeppelin stellar-contracts](https://github.com/OpenZeppelin/stellar-contracts), por ejemplo `--from oz/nft-royalties`
  - `stellar/<name>` — un contrato de [soroban-examples](https://github.com/stellar-scaffold/soroban-examples), por ejemplo `--from stellar/hello-world`
- `--ls`: Lista los ejemplos de contratos disponibles
- `--from-wizard`: Abre el Contract Wizard de OpenZeppelin en tu navegador
- `-o <output>` o `--output <output>`: Directorio de salida para el contrato generado (por defecto `contracts/<example-name>`)
- `--force`: Sobrescribe la ruta de destino si ya existe

El comando generate descarga el ejemplo, lo guarda en caché localmente, lo escribe en `contracts/<example-name>/`, y fusiona las dependencias que necesita en el `Cargo.toml` de tu workspace. Cada conjunto de ejemplos está fijado a una versión soportada en lugar de seguir el `main` upstream, así que la versión que obtienes es aquella con la que se probó esta versión de la CLI.

Si el contrato recibe argumentos de constructor, agrégalos tú mismo a `environments.toml` — `generate` no los escribe por ti.

## Comando Upgrade

Transforma un workspace de Soroban existente en un proyecto scaffold completo:

```bash
stellar scaffold upgrade [workspace-path]
```

Opciones:

- `workspace-path`: Ruta al workspace existente (por defecto el directorio actual)

El comando upgrade:

- Valida el workspace existente (requiere `Cargo.toml` y un directorio `contracts/`)
- Descarga e integra la plantilla de frontend
- Genera `environments.toml` con los contratos detectados
- Analiza los contratos en busca de argumentos de constructor y solicita su configuración
- Preserva todo el código de contrato existente y la estructura del proyecto
- Agrega herramientas y configuraciones de desarrollo

Requisitos para el upgrade:

- Debe tener un archivo `Cargo.toml` en la raíz del workspace
- Debe tener un directorio `contracts/` con contratos de Soroban
- Los contratos deben estar configurados correctamente como crates `cdylib`

## Comando Build

Compila los contratos y genera los paquetes de cliente del frontend:

```bash
stellar scaffold build [options]
```

Opciones:

- `--build-clients`: Genera paquetes de cliente de TypeScript para los contratos
- `--list` o `--ls`: Lista los nombres de los paquetes en orden de compilación
- [También se admiten las opciones estándar de compilación de contratos de Soroban]

## Comando Dev

Inicia el modo de desarrollo con recarga en caliente:

```bash
stellar scaffold watch [options]
```

Opciones:

- `--build-clients`: Genera paquetes de cliente de TypeScript mientras observa cambios
- También se admiten todas las opciones del comando build

## Comando Clean

Elimina los artefactos que Stellar Scaffold generó para un proyecto:

```bash
stellar scaffold clean [options]
```

Opciones:

- `--manifest-path <path>`: Ruta a `Cargo.toml` (por defecto el directorio actual)

Clean elimina:

- `target/stellar/`, el estado de despliegue para las redes local y de prueba
- Todo lo generado en tu directorio de clientes, dejando intactos los archivos versionados como `.gitkeep`
- Los alias de contrato e identidad que la CLI creó para las redes local y de prueba

Recurre a esto cuando quieras un despliegue verdaderamente nuevo. Debido a que Stellar Scaffold solo redespliega un contrato cuando el contrato en sí cambia, editar otra cosa — un script `after_deploy`, por ejemplo — no producirá una nueva instancia por sí solo. Limpiar los alias hace que el siguiente `stellar scaffold watch` despliegue desde cero.

## Comando Update Environment

Actualiza variables de entorno en el archivo .env:

```bash
stellar scaffold update-env --name <var-name> [options]
```

Opciones:

- `--name`: Nombre de la variable de entorno a actualizar
- `--value`: Nuevo valor (si no se proporciona, se lee desde stdin)
- `--env-file`: Ruta al archivo .env (por defecto ".env")

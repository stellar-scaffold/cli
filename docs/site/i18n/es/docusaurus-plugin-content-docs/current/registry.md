# Guía del Registry

El Stellar Registry es un sistema para publicar, desplegar y gestionar contratos inteligentes en la red Stellar. Esta guía explica cómo usar las herramientas CLI del registry para gestionar tus contratos.

## Descripción General

El sistema de registry consta de dos componentes principales:

1. **Contratos de registry on-chain** - Un registry "verificado" raíz y un registry "no verificado"
2. La herramienta CLI `stellar-registry` para interactuar con los registries

### Tipos de Registry

Hay dos tipos de registries:

- **Registry Verificado (Raíz)** - Un registry gestionado donde una cuenta administradora debe aprobar las publicaciones iniciales y los registros de nombres de contratos. Esto asegura que los contratos establecidos en el registry verificado han sido revisados.
- **Registry No Verificado** - Un registry no gestionado donde cualquiera puede publicar wasms o registrar nombres de contratos sin aprobación.

### Resolución de Nombres

Los nombres en el registry soportan prefijos de namespace. La CLI resuelve nombres usando el registry raíz como fuente de verdad:

- `mi-contrato` - Busca en el registry verificado (raíz)
- `unverified/mi-contrato` - Primero obtiene el ID del contrato de registry `unverified` del registry raíz, luego busca `mi-contrato` en ese registry

### Normalización de Nombres

Todos los nombres se normalizan antes de almacenarlos:
- Los guiones bajos (`_`) se convierten en guiones (`-`)
- Las letras mayúsculas se convierten en minúsculas
- Los nombres deben comenzar con un carácter alfabético
- Los nombres solo pueden contener caracteres alfanuméricos, guiones o guiones bajos
- Las palabras clave de Rust no están permitidas como nombres
- Los nombres tienen una longitud máxima de 64 caracteres

## Requisitos Previos

- Instalar la CLI del registry:

```bash
cargo install --git https://github.com/stellar-registry/cli stellar-registry-cli
```

## Comandos

### Publicar Contrato

Publicar un contrato compilado en el Stellar Registry:

```bash
stellar registry publish \
  --wasm <RUTA_AL_WASM> \
  [--author <DIRECCION_AUTOR>] \
  [--wasm-name <NOMBRE>] \
  [--binver <VERSION>] \
  [--dry-run]
```

Opciones:

- `--wasm`: Ruta al archivo WASM compilado (requerido)
- `--author (-a)`: Dirección del autor (opcional, por defecto la cuenta fuente configurada)
- `--wasm-name`: Nombre para el contrato publicado, soporta notación de prefijo como `unverified/mi-contrato` (opcional, extraído de los metadatos del contrato si no se proporciona)
- `--binver`: Versión binaria (opcional, extraído de los metadatos del contrato si no se proporciona)
- `--dry-run`: Simular la operación de publicación sin ejecutarla realmente (opcional)

**Nota:** Para el registry verificado, el administrador debe aprobar las publicaciones iniciales. Para el registry no verificado, usa el prefijo `unverified/`.

### Desplegar Contrato

Desplegar un contrato publicado con parámetros de inicialización opcionales:

```bash
stellar registry deploy \
  --contract-name <NOMBRE_DESPLEGADO> \
  --wasm-name <NOMBRE_PUBLICADO> \
  [--version <VERSION>] \
  [--deployer <DIRECCION_DEPLOYER>] \
  -- \
  [ARGS_CONSTRUCTOR...]
```

Opciones:

- `--contract-name`: El nombre a dar a esta instancia del contrato, soporta notación de prefijo como `unverified/mi-instancia` (requerido)
- `--wasm-name`: El nombre del contrato previamente publicado a desplegar, soporta notación de prefijo (requerido)
- `--version`: Versión específica del contrato publicado a desplegar (opcional, por defecto la versión más reciente)
- `--deployer`: Dirección opcional del deployer para resolución determinística de ID de contrato (característica avanzada)
- `ARGS_CONSTRUCTOR`: Argumentos opcionales para la función constructora

Nota: Usa `--` para separar las opciones de CLI de los argumentos del constructor.

**Nota:** Para el registry verificado, el administrador debe aprobar el despliegue con un nombre registrado. Para el registry no verificado, usa el prefijo `unverified/`.

### Desplegar Contrato Sin Nombre

Desplegar un contrato publicado sin registrar un nombre en el registry. Esto es útil cuando quieres desplegar un contrato pero no necesitas resolución de nombres:

```bash
stellar registry deploy-unnamed \
  --wasm-name <NOMBRE_PUBLICADO> \
  [--version <VERSION>] \
  [--salt <SALT_HEX>] \
  [--deployer <DIRECCION_DEPLOYER>] \
  -- \
  [ARGS_CONSTRUCTOR...]
```

Opciones:

- `--wasm-name`: El nombre del contrato previamente publicado a desplegar, soporta notación de prefijo como `unverified/mi-contrato` (requerido)
- `--version`: Versión específica del contrato publicado a desplegar (opcional, por defecto la versión más reciente)
- `--salt`: Salt opcional codificado en hex de 32 bytes para ID de contrato determinístico. Si no se proporciona, se usa un salt aleatorio
- `--deployer`: Cuenta deployer para resolución de ID de contrato determinístico (opcional)
- `ARGS_CONSTRUCTOR`: Argumentos opcionales para la función constructora

Nota: Usa `--` para separar las opciones de CLI de los argumentos del constructor.

### Registrar Contrato Existente

Registrar un nombre para un contrato existente que no fue desplegado a través del registry:

```bash
stellar registry register-contract \
  --contract-name <NOMBRE> \
  --contract-address <DIRECCION_CONTRATO> \
  [--owner <DIRECCION_PROPIETARIO>] \
  [--dry-run]
```

Opciones:

- `--contract-name`: Nombre a registrar para el contrato, soporta notación de prefijo como `unverified/mi-contrato` (requerido)
- `--contract-address`: La dirección del contrato a registrar (requerido)
- `--owner`: Propietario del registro del contrato (opcional, por defecto la cuenta fuente)
- `--dry-run`: Simular la operación sin ejecutar (opcional)

Esto te permite agregar contratos existentes al registry para resolución de nombres sin redesplegarlos.

**Nota:** Para el registry verificado, el administrador debe aprobar los registros de nombres. Usa el prefijo `unverified/` para el registry no verificado.

### Instalar Contrato

Instalar un contrato desplegado como alias para usar con `stellar-cli`:

```bash
stellar registry create-alias <NOMBRE_CONTRATO>
```

Opciones:

- `NOMBRE_CONTRATO`: Nombre del contrato desplegado a instalar, soporta notación de prefijo como `unverified/mi-contrato` (requerido)

### Publicar Hash

Publicar un hash de Wasm ya subido al registry. Esto es útil cuando ya has subido un binario Wasm usando `stellar contract upload` y quieres registrarlo en el registry:

```bash
stellar registry publish-hash \
  --wasm-hash <HASH> \
  --wasm-name <NOMBRE> \
  --version <VERSION> \
  [--author <DIRECCION_AUTOR>] \
  [--dry-run]
```

Opciones:

- `--wasm-hash`: El hash codificado en hex de 32 bytes del Wasm ya subido (requerido)
- `--wasm-name`: Nombre para el contrato publicado, soporta notación de prefijo como `unverified/mi-contrato` (requerido)
- `--version`: Cadena de versión, ej. "1.0.0" (requerido)
- `--author (-a)`: Dirección del autor (opcional, por defecto la cuenta fuente)
- `--dry-run`: Simular la operación sin ejecutar (opcional)

### Obtener ID de Contrato

Buscar el ID de un contrato desplegado por su nombre registrado:

```bash
stellar registry fetch-contract-id <NOMBRE_CONTRATO>
```

Opciones:

- `NOMBRE_CONTRATO`: Nombre del contrato desplegado, soporta notación de prefijo como `unverified/mi-contrato` (requerido)

### Obtener Hash

Obtener el hash del Wasm de un contrato publicado:

```bash
stellar registry fetch-hash <NOMBRE_WASM> [--version <VERSION>]
```

Opciones:

- `NOMBRE_WASM`: Nombre del Wasm publicado, soporta notación de prefijo como `unverified/mi-contrato` (requerido)
- `--version`: Versión específica a obtener (opcional, por defecto la última versión)

### Versión Actual

Obtener la versión actual (más reciente) de un Wasm publicado:

```bash
stellar registry current-version <NOMBRE_WASM>
```

Opciones:

- `NOMBRE_WASM`: Nombre del Wasm publicado, soporta notación de prefijo como `unverified/mi-contrato` (requerido)

### Obtener Propietario del Contrato

Buscar el propietario que registró un nombre de contrato:

```bash
stellar contract invoke --id <ID_CONTRATO_REGISTRY> -- \
  fetch_contract_owner \
  --contract-name <NOMBRE>
```

## Configuración

La CLI del registry respeta las siguientes variables de entorno:

- `STELLAR_REGISTRY_CONTRACT_ID`: Sobrescribir el ID de contrato del registry por defecto
- `STELLAR_NETWORK`: Red a usar (ej. "testnet", "mainnet")
- `STELLAR_RPC_URL`: Endpoint RPC personalizado (por defecto: https://soroban-testnet.stellar.org:443)
- `STELLAR_NETWORK_PASSPHRASE`: Passphrase de la red (por defecto: Test SDF Network ; September 2015)
- `STELLAR_ACCOUNT`: Cuenta fuente a usar

Estas variables también pueden estar en un archivo `.env` en el directorio de trabajo actual.

También puedes configurar los valores por defecto de `stellar-cli`:

```bash
stellar keys use alice
stellar network use testnet
```

## Flujo de Trabajo de Ejemplo

### Publicar en el Registry No Verificado

Para la mayoría de usuarios, el registry no verificado permite publicar sin aprobación del administrador:

1. Publicar un contrato en el registry no verificado:

```bash
stellar registry publish \
  --wasm path/to/token.wasm \
  --wasm-name unverified/mi-token \
  --binver "1.0.0"
```

2. Desplegar el contrato publicado con argumentos del constructor:

```bash
stellar registry deploy \
  --contract-name unverified/mi-instancia-token \
  --wasm-name unverified/mi-token \
  --version "1.0.0" \
  -- \
  --name "Mi Token" \
  --symbol "MTK" \
  --decimals 7
```

3. Instalar el contrato desplegado localmente:

```bash
stellar registry create-alias unverified/mi-instancia-token
```

4. Usar el contrato instalado con `stellar-cli`:

```bash
stellar contract invoke --id mi-instancia-token -- --help
```

### Publicar en el Registry Verificado

El registry verificado requiere aprobación del administrador para publicaciones iniciales. Contacta al administrador del registry para obtener aprobación para la publicación de tu contrato.

## Mejores Prácticas

1. Usa nombres descriptivos para contratos y wasms que reflejen el propósito del contrato
2. Sigue el versionado semántico para las versiones de tus contratos
3. Siempre prueba los despliegues en testnet antes de mainnet
4. Usa el flag `--dry-run` para simular operaciones antes de ejecutarlas
5. Documenta los parámetros de inicialización usados para cada despliegue
6. Usa variables de entorno o archivos `.env` para diferentes configuraciones de red

## Direcciones del Contrato Registry

El contrato de **registry verificado (raíz)** está desplegado en diferentes direcciones para cada red:

- **Testnet**: `CBFFTTX7QKA76FS4LHHQG54BC7JF5RMEX4RTNNJ5KEL76LYHVO3E3OEE`
- **Mainnet**: `CCRKU6NT4CRG4TVKLCCJFU7EOSAUBHWGBJF2JWZJSKTJTXCXXTKOJIUS`
- **Futurenet**: `CBUP2U7IY4GBZWILAGFGBOGEJEVSWZ6FAIKAX2L7PYOEE7R556LNXRJM`
- **Local**: `CDUK4O7FPAPZWAMS6PBKM7E4IO5MCBJ2ZPZ6K2GOHK33YW7Q4H7YZ35Z`

El **registry no verificado** es desplegado por el registry raíz y se puede buscar usando:

```bash
stellar contract invoke --id <ID_REGISTRY_RAIZ> -- fetch_contract_id --contract-name unverified
```

## Solución de Problemas

### Problemas Comunes

1. **El nombre del contrato ya existe**: Los nombres de contratos deben ser únicos dentro de cada registry. Elige un nombre diferente o verifica si eres propietario del contrato existente.

2. **La versión debe ser mayor que la actual**: Al publicar actualizaciones, asegúrate de que la nueva versión sigue el versionado semántico y es mayor que la versión actualmente publicada.

3. **Errores de autenticación**: Asegúrate de que tu cuenta fuente tiene suficiente balance de XLM y está correctamente configurada.

4. **Configuración de red**: Verifica que tu configuración de red coincide con el destino de despliegue deseado (testnet vs mainnet).

5. **Se requiere aprobación del administrador**: Para el registry verificado, las publicaciones iniciales y los registros de nombres de contratos requieren aprobación del administrador. Usa el prefijo `unverified/` para publicar sin aprobación.

6. **Nombre inválido**: Los nombres deben comenzar con un carácter alfabético y contener solo caracteres alfanuméricos, guiones o guiones bajos. Las palabras clave de Rust no pueden usarse como nombres.

Para información más detallada sobre los comandos disponibles:

```bash
stellar registry --help
stellar registry <command> --help
```

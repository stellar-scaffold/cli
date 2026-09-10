# Guía del Registry

Stellar Registry es un sistema para publicar, desplegar y gestionar contratos inteligentes —y sus Wasms subyacentes— en la red Stellar. Esta guía explica cómo usar la CLI y la UI de Registry para gestionar tus contratos.

<div class="videoWrapper">
  <iframe src="https://www.youtube-nocookie.com/embed/xAlWmJOdMSQ?si=n2yYDkKbyqTAhiNP" title="Recorrido completo por Stellar Registry" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>
</div>

## Descripción General

En esencia, Stellar Registry es un contrato inteligente.

### Conceptos Clave

El contrato inteligente de Registry lleva un registro de dos tipos de información:

1. **Wasms**: Los contratos inteligentes de Stellar se compilan a [WebAssembly](https://webassembly.org/), o Wasm. El archivo `.wasm` que obtienes al final de un [comando `stellar scaffold build`](./cli#comando-build) debe subirse a la blockchain de Stellar. Sin Registry, esto se hace con la CLI de Stellar usando `stellar contract upload`.

   Esto pone tu archivo/blob/binario/módulo Wasm on-chain, _identificado únicamente por su hash de contenido._ Esto se ve como una cadena de 65 caracteres hexadecimales, como `d1d4e69…`.

   Stellar Registry te permite, como autor del contrato, además darle a ese módulo Wasm un _nombre_ y una _versión._ La idea clave aquí: Stellar _ya es un sistema de distribución de módulos_, como NPM o Crates.io. Lo que pasa es que, sin Registry, es inutilizable. Registry provee la capa de nombre+versión+búsqueda que revela esta verdad y la hace útil.

   Por esta razón, Stellar Registry prefiere el verbo _publicar_, en lugar del más genérico "subir". No solo estás subiendo un blob Wasm a la blockchain; estás publicando un módulo. Como en otros sistemas de distribución de módulos, luego mantienes el control de ese nombre y los derechos de publicar nuevas versiones en la serie.

2. **Contratos:** Una vez que un Wasm está on-chain, cualquiera puede desplegar cualquier cantidad de contratos que usen ese mismo Wasm subyacente. Si estás familiarizado con la Programación Orientada a Objetos, piensa en el Contrato como la _instancia_ y en el Wasm como la _clase._ Un Wasm define el comportamiento; un Contrato guarda datos (incluyendo una referencia al Wasm).

   Como ejemplo concreto, piensa en un Wasm como [`oz/ft-standard`](https://stellar.rgstry.xyz/wasms/oz/ft-standard) —la implementación estándar de Token Fungible de Open Zeppelin—. Podrías crear tres tokens separados a partir de este único Wasm simplemente desplegando tres contratos inteligentes distintos y dándoles diferentes parámetros de inicialización (quizás eres un estafador y por eso los llamas "USDC", "EURC" y "BTC" —nada te lo impide—). Todos estos _Contratos_ referencian el mismo _Wasm_ y por lo tanto tienen el mismo comportamiento, la misma interfaz, los mismos métodos; solo difieren en su _almacenamiento_: en sus parámetros de inicialización y en las distintas interacciones (depósitos, retiros, transferencias) que ajustan sus datos guardados.

   Aunque Stellar Registry le da a los Wasms tanto nombres como _versiones_, tiene menos sentido que los Contratos tengan versiones. Un contrato no es un módulo; es más parecido a una app o un almacén de datos. Siempre interactúas con la versión activa y más reciente de un Contrato.

Eso es todo, en esencia: Stellar Registry es un contrato inteligente que le da nombres a otros contratos inteligentes, y que le da a los Wasms tanto nombres como versiones.

### Enlaces Rápidos

Alrededor del contrato inteligente principal, Stellar Registry envuelve otras herramientas:

- [Stellar Registry UI](https://rgstry.xyz), `rgstry.xyz`, donde puedes explorar y buscar todos los Wasms y Contratos registrados actualmente. [Código fuente](https://github.com/stellar-registry/ui)
- [Stellar Registry CLI](https://crates.io/crates/stellar-registry-cli), la interfaz principal usada hoy para _escribir_ en Stellar Registry. Esta es la herramienta que usas para _publicar Wasms_ y _desplegar Contratos_ en Stellar Registry, o para registrar Wasms ya subidos y Contratos ya desplegados. [Código fuente](https://github.com/stellar-registry/cli)
- [Macros de Rust de Stellar Registry](https://crates.io/crates/stellar-registry), para simplificar tus llamadas entre contratos al escribir contratos. [Código fuente](https://github.com/stellar-registry/cli/tree/main/crates/stellar-registry-macro)
- [Stellar Registry GitHub Actions](https://github.com/stellar-registry/actions), para crear [builds atestiguados SEP-55](https://github.com/stellar-expert/soroban-build-workflow) de los archivos Wasm de tu proyecto, subirlos a la blockchain, incrementar la versión de tu serie en Registry, y auto-publicar tu Wasm en Registry. [Próximamente](https://github.com/stellar-registry/oz-combined-wasms/issues/1)
- **Gobernanza de Stellar Registry**: para conseguir que tus Contratos y Wasms entren al registry raíz (sin el prefijo `unverified/`; más sobre esto abajo) o para conseguir tu propio subregistro (tu propio prefijo, como `oz/` o `circle/`, en el que puedes publicar Wasms y desplegar Contratos libremente), necesitas presentar una solicitud ante el consejo de seguridad de Stellar Registry. La UI de Stellar Registry contiene formularios para hacer esto ([próximamente](https://github.com/stellar-registry/ui/issues/51)), pero si quieres hacerlo directamente en el sistema subyacente:
  - en Testnet, [usa Tansu](https://testnet.tansu.dev/governance/?name=stellarregistry)
  - en Mainnet, usa [github.com/stellar-registry/gov](https://github.com/stellar-registry/gov/issues) (Tansu próximamente)
- API de Stellar Registry para [testnet](https://stellar-registry-testnet.fly.dev/) y [mainnet](https://stellar-registry-mainnet.fly.dev/), los backends de la UI de Stellar Registry. Úsala bajo tu propio riesgo: los dominios pueden cambiar y los esquemas de limitación de tasa pueden endurecerse. [Código fuente](https://github.com/stellar-registry/indexer)

A continuación, aprendamos más sobre ese prefijo `unverified/`.

### Subregistros

¿Dijimos que Registry era _un solo_ contrato? Eso no es del todo cierto. El Registry _raíz_ es un contrato, pero algunos de los contratos que rastrea son a su vez Registries, ejecutando el mismo [Wasm de Registry](https://stellar.rgstry.xyz/wasms/registry). Estos son _subregistros_.

La mayoría, como el Registry raíz, son Registries bloqueados y _gestionados_. No puedes publicar Wasms ni desplegar Contratos en los subregistros `oz` o `circle`, por razones obvias de seguridad.

El subregistro `unverified`, sin embargo, _no está gestionado._ Cualquiera puede publicar y desplegar en él. Como su nombre indica, _no deberías confiar en los Wasms y Contratos de este subregistro._ Si ves el prefijo `unverified` en el nombre de un Wasm o Contrato, _¡ten cuidado!_

Algunos subregistros populares, para ayudarte a entender cómo funciona todo esto:

- [`oz/`](https://stellar.rgstry.xyz/wasms?query=oz), los Wasms de Open Zeppelin (gestionado [por el equipo de Stellar Registry](https://stellar.rgstry.xyz/wasms?query=oz))
- [`circle/`](https://stellar.rgstry.xyz/contracts?query=circle), que contiene los tokens oficiales [`usdc`](https://stellar.rgstry.xyz/contracts/circle/usdc) y [`eurc`](https://stellar.rgstry.xyz/contracts/circle/eurc)
- `unverified/`: échale un vistazo a [Wasms de Testnet](https://testnet.rgstry.xyz/wasms?query=unverified) y [Contratos de Testnet](https://testnet.rgstry.xyz/contracts?query=unverified), así como a [Wasms de Mainnet](https://stellar.rgstry.xyz/wasms?query=unverified) y [Contratos de Mainnet](https://stellar.rgstry.xyz/contracts?query=unverified) para hacerte una idea de lo ruidoso que puede ser este subregistro.

Cuando publiques o despliegues por primera vez en Stellar Registry, necesitarás usar el subregistro `unverified`.

Nunca esto:

```
# ❌ NO FUNCIONA
stellar registry publish --wasm-name my-wasm
```

En cambio, esto:

```
# ✅ FUNCIONA
stellar registry publish --wasm-name unverified/my-wasm
```

:::tip

Para ver un ejemplo completo y funcional del comando `publish`, [mira más abajo](#usando-el-registry-no-verificado).

:::

Luego, cuando estés listo, puedes usar el [proceso de gobernanza enlazado arriba](#enlaces-rápidos) para entrar al Registry raíz, a un subregistro diferente, o para conseguir tu propio subregistro.

### Resolución de Nombres

Los nombres en el registry soportan prefijos de namespace. La CLI resuelve nombres usando el registry raíz como fuente de verdad:

- `mi-contrato` - Busca en el registry verificado (raíz)
- `unverified/mi-contrato` - Primero obtiene el ID del contrato de registry `unverified` del registry raíz, luego busca `mi-contrato` en ese registry
- `otro-subregistro/mi-contrato` — Igual que arriba. Busca `otro-subregistro` en el registry raíz, luego busca `mi-contrato` en ese registry.

### Normalización de Nombres

Todos los nombres son normalizados por el crate [stellar-registry-names](https://crates.io/crates/stellar-registry-name) antes de almacenarse:

- Los guiones bajos (`_`) se convierten en guiones (`-`)
- Las letras mayúsculas se convierten en minúsculas
- Los nombres deben comenzar con un carácter alfabético
- Los nombres solo pueden contener caracteres alfanuméricos, guiones o guiones bajos
- Las palabras clave de Rust no están permitidas como nombres
- Los nombres tienen una longitud máxima de 64 caracteres

## Requisitos Previos

Instala la CLI del registry:

```bash
cargo install --locked stellar-registry-cli
```

Como actualmente incluye la CLI de Stellar como dependencia, toma algo de tiempo. Para acelerarlo, puedes usar [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```bash
cargo install cargo-binstall
cargo binstall stellar-registry-cli
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

**Nota:** Para el registry raíz, el proceso de gobernanza ([ver arriba](#enlaces-rápidos)) debe aprobar las publicaciones iniciales. Para el registry no verificado, usa el prefijo `unverified/`.

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

**Nota:** Para el registry raíz, el proceso de gobernanza ([ver arriba](#enlaces-rápidos)) debe aprobar las publicaciones iniciales. Para el registry no verificado, usa el prefijo `unverified/`.

### Desplegar Contrato Sin Nombre

Desplegar un contrato publicado sin registrar un nombre en el registry. Esto es útil cuando quieres desplegar un contrato pero no necesitas resolución de nombres. Puedes hacerlo con el botón "Deploy a contract using this Wasm" en cualquier página de Wasm en https://rgstry.xyz, o usando la CLI:

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

**Nota:** Para el registry raíz, el proceso de gobernanza ([ver arriba](#enlaces-rápidos)) debe aprobar las publicaciones iniciales. Para el registry no verificado, usa el prefijo `unverified/`.

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

**Nota:** Para el registry raíz, el proceso de gobernanza ([ver arriba](#enlaces-rápidos)) debe aprobar las publicaciones iniciales. Para el registry no verificado, usa el prefijo `unverified/`.

### Obtener ID de Contrato

Buscar el ID de un contrato desplegado por su nombre registrado:

```bash
stellar registry fetch-contract-id <NOMBRE_CONTRATO>
```

Opciones:

- `NOMBRE_CONTRATO`: Nombre del contrato desplegado, soporta notación de prefijo como `unverified/mi-contrato` (requerido)

### Crear Alias de Contrato

Un patrón común es usar `fetch-contract-id` (ver arriba) y luego `stellar contract alias` para crear un alias local del contrato nombrado. `create-alias` hace eso en un solo comando:

```bash
stellar registry create-alias <NOMBRE_CONTRATO> [NOMBRE_LOCAL]
```

Ejemplo:

```bash
stellar registry create-alias circle/usdc
```

Esto crea el alias de contrato `usdc` (sin el prefijo `circle`), que luego puedes usar para transferir activos:

```bash
stellar contract invoke --id usdc -- transfer \
    --from account-1 \ # creada con `stellar keys`
    --to account-2 \
    --amount 10000000 # 1 USDC; siempre verifica el `decimals` de un activo antes de enviar
```

:::caution ¡Cuidado con los SAC!

El ejemplo de transferencia de `usdc` de arriba usa `contract invoke`, que utiliza el wrapper/interfaz de Soroban para el token USDC. Este wrapper/interfaz de Soroban se llama Stellar Asset Contract (SAC). No todos los sistemas heredados detectan las transferencias hechas vía SAC. Siempre prueba las transferencias con montos pequeños y verifica que tu sistema de destino funcione como se espera. Si necesitas usar una transacción de Stellar Classic, el equivalente a lo anterior sería:

```bash
stellar tx new payment --asset USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN \
    --source account-1 \
    --destination account-2 \
    --amount 10000000
```

:::

Opciones:

- `NOMBRE_CONTRATO`: Nombre del contrato desplegado, soporta notación de prefijo como `unverified/mi-contrato` (requerido).
- `NOMBRE_LOCAL`: Nombre local personalizado opcional para el alias. Si no se proporciona, usa el nombre del registry.
- `--force` / `-f`: Fuerza la sobrescritura si ya existe un alias con el mismo nombre, y permite crear un alias para un contrato marcado como comprometido en el registry.

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

Estas variables también pueden estar en un archivo `.env` en el directorio de trabajo actual, si tu shell está configurado para respetar `.env`.

También puedes configurar los valores por defecto de `stellar-cli`:

```bash
stellar keys use alice
stellar network use testnet
```

## Flujo de Trabajo de Ejemplo

### Usando el Registry No Verificado

Para la mayoría de usuarios, el registry no verificado permite publicar sin aprobación del administrador:

#### 1. Publica un contrato en el registry no verificado:

```bash
stellar registry publish \
  --wasm path/to/token.wasm \
  --wasm-name unverified/mi-token \
  --binver "1.0.0"
```

#### 2. Registra un Wasm ya subido en el registry no verificado:

```bash
stellar registry publish-hash \
  --wasm-hash d1d4e69… \
  --wasm-name unverified/mi-token \
  --version "1.0.0"
```

#### 3. Despliega un contrato con argumentos de constructor a partir de un Wasm que ya está en Registry:

```bash
stellar registry deploy \
  --contract-name unverified/mi-token \
  --wasm-name oz/ft-standard \
  --version "1.0.0" \
  -- \
  --name "Mi Token" \
  --symbol "MTK" \
  --decimals 7 \
  --owner G123… \
  --initial_supply 100
```

#### 4. Registra un Contrato ya desplegado en el registry no verificado:

```bash
stellar registry register-contract \
  --contract-name <NOMBRE> \
  --contract-address <DIRECCION_CONTRATO> \
```

### Usando un Registry Verificado

Un registry verificado, como el registry raíz, requiere aprobación del administrador para las publicaciones y despliegues iniciales. Usa el proceso de gobernanza [descrito arriba](#enlaces-rápidos) para conseguir que tu contrato sea aprobado para su publicación.

## Mejores Prácticas

1. Usa nombres descriptivos para contratos y wasms que reflejen el propósito del contrato
2. Sigue el versionado semántico para las versiones de tus contratos
3. Siempre prueba los despliegues en testnet antes de mainnet
4. Usa el flag `--dry-run` para simular operaciones antes de ejecutarlas
5. Documenta los parámetros de inicialización usados para cada despliegue
6. Usa configuraciones explícitas de `--source` y `--network` en cada comando, en lugar de depender de la configuración de `stellar network use` y `stellar keys use`

## Solución de Problemas

### Problemas Comunes

1. **El nombre del contrato ya existe**: Los nombres de contratos deben ser únicos dentro de cada registry. Elige un nombre diferente o verifica si eres propietario del contrato existente.

2. **La versión debe ser mayor que la actual**: Al publicar actualizaciones, asegúrate de que la nueva versión sigue el versionado semántico y es mayor que la versión actualmente publicada.

3. **Errores de autenticación**: Asegúrate de que tu cuenta fuente tiene suficiente balance de XLM y está correctamente configurada.

4. **Configuración de red**: Verifica que tu configuración de red coincide con el destino de despliegue deseado (testnet vs mainnet).

5. **Se requiere aprobación del administrador**: Para el registry verificado, las publicaciones iniciales y los registros de nombres de contratos requieren aprobación del administrador (ver la sección de "gobernanza" [arriba](#enlaces-rápidos)). Usa el prefijo `unverified/` para publicar sin aprobación.

6. **Nombre inválido**: Los nombres deben comenzar con un carácter alfabético y contener solo caracteres alfanuméricos, guiones o guiones bajos. Las palabras clave de Rust no pueden usarse como nombres.

Para información más detallada sobre los comandos disponibles:

```bash
stellar registry --help
stellar registry <command> --help
```

# Despliegue

## Contrato inteligente

Cuando estés listo para testnet/mainnet, recomendamos desplegar tu contrato usando `stellar registry`. El registry tiene dos namespaces:

- **Registry verificado** (por defecto) - Requiere aprobación del administrador para publicaciones y despliegues iniciales
- **Registry no verificado** - Abierto para que cualquiera publique (usa el prefijo `unverified/`)

Algunos comandos para comenzar:

```bash
#  Nota: el argumento --source-account se omite por claridad

# Primero publica tu contrato en el registry no verificado
stellar registry publish \
  --wasm target/stellar/local/my_contract.wasm \
  --wasm-name unverified/my-contract \
  --binver "1.0.0"

# Luego despliega una instancia con parámetros del constructor
stellar registry deploy \
  --contract-name unverified/my-contract-instance \
  --wasm-name unverified/my-contract \
  -- \
  --param1 value1

# Puedes acceder a la documentación de ayuda con --help
stellar registry deploy \
  --contract-name unverified/my-contract-instance \
  --wasm-name unverified/my-contract \
  -- \
  --help

# Instala el contrato desplegado localmente
stellar registry create-alias unverified/my-contract-instance
```

**Nota:** Los nombres se normalizan - los guiones bajos se convierten en guiones y las mayúsculas en minúsculas.

Además, podrías querer registrar tu contrato en [Stellar.Expert](https://stellar.expert/explorer/public/contract/validation).

Proporcionamos una GitHub action de plantilla que debes ajustar según tus necesidades. Esta action compila tu(s) contrato(s), crea algunas attestations firmadas y registra el Wasm específico en Stellar.Expert. Luego debes descargar y subir ese Wasm específico usando los comandos de `stellar registry` mencionados anteriormente.

## dApp

Una vez que estés listo para desplegar tu dApp, puedes ejecutar:

```bash
npm run build
```

Esto empaquetará tu aplicación en la carpeta `/dist`. Luego tienes principalmente dos enfoques para servir su contenido:

1. Un proveedor de servicio centralizado como GitHub Pages, Vercel, Netlify, etc.
2. Una solución descentralizada como IPFS.

Como estamos desarrollando una aplicación blockchain y la descentralización es uno de los principios fundamentales de lo que hacemos, recomendamos desplegar tu aplicación siguiendo un enfoque descentralizado.

Si por alguna razón aún prefieres una solución centralizada, puedes consultar esta extensa guía de [Vite](https://vite.dev/guide/static-deploy).

El resto de esta guía se enfocará en un despliegue descentralizado. Proporcionamos un flujo de trabajo de GitHub action que compila la aplicación, crea una attestation firmada y despliega los archivos en IPFS:

1. Haz push en `dapp_production`,
2. El flujo de trabajo de IPFS se ejecuta y produce una dirección IPFS (CID),

El CID puede usarse con cualquier gateway de IPFS. Esto podría no parecer muy conveniente para tus usuarios; afortunadamente, en el ecosistema tenemos un gateway con el que es muy fácil integrarse:

https://xlm.sh/

Gracias a ese proveedor, puedes tener un sitio web bajo `tu-dapp.xlm.sh`.

IPFS no es mágico en el sentido de que tenemos que hacer alguna configuración. Necesitas un punto de entrada al que subir tus archivos si tú mismo no quieres ejecutar un nodo IPFS. Nuestra GitHub action se integra con Storacha, y a continuación hay algunas instrucciones.

### Configuración de Storacha

Sigue la [documentación](https://docs.storacha.network) de Storacha para crear un Space y obtener su Proof (una cadena de prueba UCAN). A continuación hay un inicio rápido.

Si no tienes una cuenta, se te pedirá que crees una y selecciones un plan. El plan gratuito es más que suficiente para lo que queremos hacer.

Instala la CLI:

```bash
npm install @storacha/cli
```

Crea una cuenta o inicia sesión:

```bash
storacha login
```

Crea un space:

```bash
storacha space create scaffold
```

Esto generará un space que se identifica mediante un DID, algo como `did:key:z6Mk...`.

El siguiente paso es crear una key; esto es `STORACHA_PRINCIPAL`:

```bash
storacha key create --json
```

La key en sí tiene su propio DID. Usa ese DID para crear una prueba de delegación:

```bash
export AUDIENCE=did:key:z6Mk...
storacha delegation create $AUDIENCE -c space/blob/add -c space/index/add -c filecoin/offer -c upload/add --base64
```

Esto es tu `STORACHA_PROOF`.

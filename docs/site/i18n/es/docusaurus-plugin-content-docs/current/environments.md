# Configuración de Entornos

Stellar Scaffold usa un archivo `environments.toml` para gestionar diferentes entornos de despliegue y configuraciones de contratos.

## Dos archivos de configuración

Un proyecto generado con scaffold tiene dos archivos de configuración en su raíz, y responden a preguntas diferentes:

| Archivo | Responde |
| --- | --- |
| `scaffold.yml` | _Dónde_ viven las cosas — de qué directorios lee los contratos la CLI y a cuáles escribe los clientes |
| `environments.toml` | _Qué_ desplegar — redes, cuentas y contratos, por entorno |

La mayor parte de esta página trata sobre `environments.toml`. `scaffold.yml` es breve y rara vez lo cambiarás:

```yaml
version: 1

config:
  contracts_dir: contracts
  clients_dir: app-lib/clients
```

- `version` es la versión del esquema. Es obligatoria, y la versión `1` es el único valor que acepta esta CLI.
- `contracts_dir` es donde viven tus crates de contratos en Rust. Por defecto: `contracts`.
- `clients_dir` es donde se escriben tus clientes de contrato generados: un paquete por contrato en `clients_dir/<name>/`, más un `clients_dir/index.ts` que tu app importa como `@stellar-scaffold/app-lib/clients`. Por defecto: `app-lib/clients`. Todo aquí se regenera en cada compilación, así que los cambios que hagas a mano se sobrescribirán.

Ambas claves son opcionales y recurren a los valores por defecto anteriores, así que un `scaffold.yml` que contenga solo `version: 1` es válido.

## Estructura del Archivo de Configuración

```toml
[development]
network = {
    name = "local",                 # usar la red local
    run_locally = true              # iniciar el contenedor Docker local
}
accounts = ["account1", "account2"] # Alias de cuentas a crear

[staging]
network = {
    name = "testnet",               # Usar la testnet de Stellar
}

[production]
network = {
    name = "mainnet",               # Usar la mainnet de Stellar
}
```

## Configuración de Red

Cada entorno puede especificar configuraciones de red:

```toml
network = {
    name = "<network-name>",           # Opcional: Usar red predefinida (mainnet/testnet/local)
    rpc_url = "<url>",                # Opcional: Endpoint RPC personalizado
    network_passphrase = "<phrase>",   # Opcional: Passphrase de red
    rpc_headers = [["key", "value"]], # Opcional: Encabezados RPC personalizados
    run_locally = false               # Opcional: Si se debe ejecutar la red local (por defecto: false)
}
```

## Configuración de Cuentas

Configura cuentas para el despliegue y prueba de contratos:

```toml
accounts = [
    "account1",                        # Alias de cuenta simple
    { name = "admin", default = true } # Cuenta con configuraciones adicionales
]
```

## Configuración de Contratos

Configura contratos inteligentes para cada entorno:

```toml
[development.contracts.my_contract]
client = true                      # Generar cliente TypeScript (por defecto: true)
constructor_args = """             # Script de inicialización si se necesita
    --arg1 param1 --arg2 param2
"""
after_deploy = """                 # lógica de invocación de configuración del contrato después del despliegue inicial
    STELLAR_ACCOUNT=admin fund --to admin --amount 100
"""

[production.contracts.my_contract]
id = "C..."                        # ID del contrato para producción/staging
client = true                      # Generar cliente TypeScript
```

### Opciones de Configuración

#### `client` (booleano, por defecto: true)

- Controla si se genera un paquete de cliente de TypeScript para este contrato
- Establécelo en `false` para omitir la generación de cliente en contratos utilitarios

```toml
[development.contracts.my_contract]
client = false  # Omitir la generación de cliente TypeScript
```

#### `id` (string, opcional)

- Especifica un ID de contrato fijo para el contrato
- Requerido en entornos de producción/staging
- Debe ser un ID de contrato de Stellar válido

```toml
[production.contracts.my_contract]
id = "C..."  # Usar un ID de contrato específico
```

#### `constructor_args` (string, opcional)

- Argumentos pasados al constructor del contrato durante el despliegue
- Se ejecuta como parte de la transacción de despliegue
- Una sola línea de argumentos separados por espacios
- Puede usar `STELLAR_ACCOUNT=<alias>` para especificar la cuenta que despliega
- Admite sustitución de comandos con `$(command)`

```toml
[development.contracts.my_contract]
constructor_args = "--arg1 1000 --account $(stellar keys address admin)"  # Argumentos básicos

# Con una cuenta específica que despliega
constructor_args = "STELLAR_ACCOUNT=admin --arg1 value1 --arg2 value2"

# Con sustitución de comandos
constructor_args = "--account1 $(stellar keys address user1) --account2 $(stellar keys address user2)"
```

#### `after_deploy` (string, opcional)

- Script de inicialización que se ejecuta después del despliegue del contrato
- Solo se ejecuta en entornos de desarrollo/pruebas
- Admite múltiples comandos en líneas separadas
- Puede usar `STELLAR_ACCOUNT=<alias>` para especificar la cuenta de origen
- Admite sustitución de comandos con `$(command)`

```toml
[development.contracts.my_contract]
after_deploy = """
# Inicialización básica
initialize --param1 value1 --param2 value2

# Usar una cuenta específica
STELLAR_ACCOUNT=admin set_admin --admin "new_admin"

# Sustitución de comandos
set_value --value "$(stellar keys address admin)"

# Múltiples operaciones
create_pool --name "Pool A"
add_liquidity --amount 1000
set_fee_rate --rate 0.003
"""
```

### Ejemplos de Configuración

```toml
# Contrato de token con argumentos de constructor
[development.contracts.token]
client = true
constructor_args = "--name Token --symbol TKN --decimals 8"

# Contrato desplegado por el admin con argumentos dinámicos
[development.contracts.marketplace]
client = true
constructor_args = "STELLAR_ACCOUNT=admin --treasury-account $(stellar keys address treasury)"

# Contrato con argumentos de constructor y script after_deploy
[development.contracts.game]
client = true
constructor_args = "STELLAR_ACCOUNT=admin --name GameV1 --start 1000"
after_deploy = """
    # Configuración adicional después del despliegue
    add_player --address "$(stellar keys address player1)"
    set_difficulty --difficulty 3
"""

# Entorno de producción con ID de contrato fijo
[production.contracts.token]
client = true
id = "CC5YYARE2TSLA..."  # Debe ser un ID de contrato válido

# Contrato utilitario sin generación de cliente
[development.contracts.utils]
client = false

# Inicialización compleja con múltiples cuentas
[development.contracts.marketplace]
client = true
after_deploy = """
    # Configurar admin
    STELLAR_ACCOUNT=admin set_admin_account --account "$(stellar keys address admin)"

    # Configurar tarifas
    STELLAR_ACCOUNT=admin set_fee_rate --rate 250

    # Agregar listado inicial
    STELLAR_ACCOUNT=seller create_listing --name "Item A" --price 1000
"""
```

## Variables de Entorno

- `STELLAR_SCAFFOLD_ENV`: Establece el entorno actual (development/testing/staging/production)
- `STELLAR_ACCOUNT`: Cuenta por defecto para transacciones (se establece automáticamente)
- `STELLAR_RPC_URL`: URL del endpoint RPC (se establece a partir de la configuración de red)
- `STELLAR_NETWORK_PASSPHRASE`: Passphrase de la red (se establece a partir de la configuración de red)

## Uso

1. Crea `environments.toml` en la raíz de tu proyecto
2. Configura entornos, redes y contratos
3. Establece `STELLAR_SCAFFOLD_ENV` para elegir el entorno
4. Usa `stellar scaffold build` o `stellar scaffold watch` para desplegar y generar clientes

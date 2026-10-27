# Live Activities: la API local

Cualquier script o app de tu máquina puede publicar actividades en la isla: un build,
un deploy, una descarga, o lo que está haciendo Claude Code.

## Conexión

| Dato   | Valor                                                                           |
| ------ | ------------------------------------------------------------------------------- |
| URL    | `http://127.0.0.1:47823/v1` (solo localhost)                                    |
| Puerto | `apiPort` en `settings.json` si el 47823 está ocupado                           |
| Token  | `%APPDATA%\dev.notchwindows.island\api-token` (se genera en el primer arranque) |
| Header | `Authorization: Bearer <token>` en **todas** las rutas                          |

Desde la bandeja, _Abrir carpeta de configuración_ te lleva directo al token y a
`settings.json`.

> Usa `127.0.0.1`, no `localhost`: la isla solo escucha en IPv4 y `localhost` puede
> resolver primero a `::1`.

## Rutas

| Método   | Ruta                  | Qué hace                                   | Respuesta |
| -------- | --------------------- | ------------------------------------------ | --------- |
| `GET`    | `/v1/health`          | Comprueba que la isla está viva            | `200`     |
| `PUT`    | `/v1/activities/{id}` | Crea o reemplaza la actividad `id`         | `204`     |
| `DELETE` | `/v1/activities/{id}` | Quita la actividad `id` (si no existe, ok) | `204`     |

`id`: de 1 a 64 caracteres; letras, números, `-`, `_`, `.` o `:`. Publicar el mismo id
la reemplaza, así que un script puede ir actualizando su progreso.

### Cuerpo del `PUT`

```json
{
  "title": "Build",
  "subtitle": "notch-windows · main",
  "state": "running",
  "progress": 0.4,
  "icon": "build",
  "ttlSeconds": 30
}
```

| Campo        | Tipo       | Requerido | Notas                                                                         |
| ------------ | ---------- | --------- | ----------------------------------------------------------------------------- |
| `title`      | string     | sí        | 1–80 caracteres.                                                              |
| `subtitle`   | string     | no        | Hasta 120 caracteres.                                                         |
| `state`      | string     | no        | `running` (default), `attention`, `success` o `error`.                        |
| `progress`   | número 0–1 | no        | Sin él, el progreso es indeterminado (spinner).                               |
| `icon`       | string     | no        | `claude`, `build`, `deploy`, `download`, `test` o `generic` (default).        |
| `ttlSeconds` | entero     | no        | Se quita sola después de N segundos. `success`/`error` usan 6 s si no lo das. |

Los campos desconocidos se rechazan con `400`, para que un error de dedo no pase
desapercibido.

**Prioridad:** `attention` le gana a la música (la música pasa a la burbuja); el resto
de los estados queda arriba de la música que suena, pero abajo de los avisos del sistema
(volumen, Bloq Mayús…).

### Errores

Siempre JSON: `{"error": "mensaje"}`.

| Código | Cuándo                                                       |
| ------ | ------------------------------------------------------------ |
| `400`  | Cuerpo inválido, campo desconocido o id con formato inválido |
| `401`  | Falta el token o no coincide                                 |
| `403`  | La petición trae header `Origin` (viene de un navegador)     |
| `404`  | Ruta desconocida                                             |
| `405`  | Método no permitido en esa ruta                              |
| `413`  | Cuerpo de más de 16 KB                                       |

## Ejemplos

### Node (el cliente incluido)

```bash
node scripts/island.mjs set build --title "Build" --subtitle main --progress 0.4 --icon build
node scripts/island.mjs set build --title "Build listo" --state success
node scripts/island.mjs clear build
```

O desde tu propio script:

```js
import { setActivity, clearActivity } from "./scripts/island.mjs";
await setActivity("deploy", { title: "Deploy a prod", icon: "deploy", progress: 0.7 });
```

### curl

```bash
TOKEN=$(cat "$APPDATA/dev.notchwindows.island/api-token")
curl -X PUT http://127.0.0.1:47823/v1/activities/build \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"Build","progress":0.5,"icon":"build"}'
```

### PowerShell

Windows PowerShell 5.1 manda el cuerpo como ISO-8859-1: si el texto lleva acentos, manda
bytes UTF-8.

```powershell
$token = (Get-Content "$env:APPDATA\dev.notchwindows.island\api-token" -Raw).Trim()
$body = '{"title":"Compilación","icon":"build"}'
Invoke-RestMethod -Method Put -Uri http://127.0.0.1:47823/v1/activities/build `
  -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json; charset=utf-8" `
  -Body ([Text.Encoding]::UTF8.GetBytes($body))
```

## Claude Code en la isla

`scripts/claude-hook.mjs` traduce los eventos de los hooks de Claude Code:

| Evento             | En la isla                                          |
| ------------------ | --------------------------------------------------- |
| `UserPromptSubmit` | "Claude trabajando" + el nombre del proyecto        |
| `Notification`     | "Claude te necesita" en ámbar (le gana a la música) |
| `Stop`             | "Claude terminó" en verde, se va sola en 8 s        |
| `SessionEnd`       | Se quita                                            |

Cada sesión es su propia actividad, así que dos Claudes en paralelo se ven por separado.
El hook nunca frena a Claude: si la isla está apagada, termina en silencio.

Agrega esto a `~/.claude/settings.json` (o al `.claude/settings.json` de un proyecto),
cambiando la ruta por la de tu clon:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node C:/ruta/a/notch_windows/scripts/claude-hook.mjs",
            "timeout": 5
          }
        ]
      }
    ],
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node C:/ruta/a/notch_windows/scripts/claude-hook.mjs",
            "timeout": 5
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node C:/ruta/a/notch_windows/scripts/claude-hook.mjs",
            "timeout": 5
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node C:/ruta/a/notch_windows/scripts/claude-hook.mjs",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
```

## CI de GitHub Actions

```bash
node scripts/ci-watch.mjs          # último run de la rama actual
node scripts/ci-watch.mjs 123456   # un run concreto
```

Muestra `CI 3/5` con progreso por jobs y, al terminar, "CI en verde" o el resultado en
rojo durante 15 s. Requiere `gh` autenticado. Consulta cada 10 s para no gastar el rate
limit de GitHub.

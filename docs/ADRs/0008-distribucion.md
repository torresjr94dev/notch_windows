# ADR-0008: Distribución (instalador, firma y actualizaciones)

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

Hasta la 0.1.0, la isla solo corría con `npm run tauri dev`. Para instalarla hace falta
decidir qué instalador generar, dónde se compila, si se firma y cómo se actualiza.

## Decisión

### Solo NSIS, por usuario

`bundle.targets = ["nsis"]` con `installMode: currentUser`: se instala en
`%LOCALAPPDATA%\Notch Windows` sin pedir permisos de administrador. Es lo natural para
una app que vive en la sesión del usuario (bandeja, clave `Run` de HKCU, token en su
`%APPDATA%`).

Sin MSI: obliga a instalar para toda la máquina (con admin), no aporta nada a una app
por usuario y WiX necesita VBScript, que Windows 11 ya trae deshabilitado. Si algún día
hace falta desplegar por GPO, se agrega como segundo target.

### Se compila en GitHub Actions, no en la máquina de nadie

`.github/workflows/release.yml` corre al subir un tag `vX.Y.Z`: valida que el tag
coincida con la versión de `tauri.conf.json` y que `CHANGELOG.md` tenga esa sección,
compila en un Windows limpio y publica el release con las notas del CHANGELOG. Usa
`npm run tauri build` y `gh release create` directamente, sin `tauri-action`: son dos
comandos y así cada paso queda a la vista.

### Sin firma de código (por ahora)

Un certificado de firma cuesta dinero cada año y exige validar la identidad.
Sin firma, SmartScreen muestra _"Windows protegió tu PC"_ hasta que el instalador gana
reputación. Las notas del release explican cómo continuar y publican el **SHA-256**
del instalador para comprobar que es el que compiló el workflow.

Revisar si la app empieza a distribuirse fuera de un círculo de confianza: Azure
Trusted Signing es la opción más barata.

### Sin auto-actualización (por ahora)

`tauri-plugin-updater` necesita su propio par de llaves de firma, un `latest.json`
publicado en cada release y UI para avisar. Para la 0.1.0 basta con instalar la versión
nueva encima: NSIS reemplaza el ejecutable y lo que vive en `%APPDATA%`
(preferencias, token de la API) se conserva.

### Una sola instancia, solo en release

`tauri-plugin-single-instance` evita dos islas encimadas (por ejemplo, _Iniciar con
Windows_ más el acceso directo de Inicio). En desarrollo no se registra: la versión
instalada y `tauri dev` comparten identifier, y `tauri dev` se cerraría en silencio.

## Consecuencias

- Publicar una versión es: subir el número en `package.json`, `Cargo.toml` y
  `tauri.conf.json`, cerrar su sección en el CHANGELOG, fusionar el PR a `main` y
  subir el tag.
- Para desarrollar con la versión instalada abierta, primero hay que salir de ella
  desde la bandeja: si no, las dos islas se enciman y la de desarrollo se queda sin API
  porque el puerto ya está ocupado.
- _Iniciar con Windows_ registra la ruta del ejecutable que lo activó. La bandeja solo
  lo marca si apunta a esa misma copia, así que al pasar de `tauri dev` a la versión
  instalada hay que volver a activarlo.
- Desinstalar no borra el valor de `Run`: si _Iniciar con Windows_ estaba activo,
  queda una entrada que apunta a un archivo que ya no existe, y Windows la ignora al
  iniciar sesión. Lo limpio es desactivarlo antes de desinstalar.

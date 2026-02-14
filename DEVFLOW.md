# 🛠 ARTHEMA DEVELOPMENT FLOW (TDD)

Para garantizar la estabilidad de **Arthema** (Cyberpunk API IDE), todo desarrollo debe seguir este flujo de trabajo obligatorio.

## 📜 Reglas de Oro
1. **TDD Primero:** No se escribe código de funcionalidad sin antes tener un test que falle.
2. **Validación Total:** Antes de cualquier publicación, el comando `cargo test` debe devolver `ok`.
3. **Cero Warnings:** No se permiten publicaciones con advertencias del compilador.

## 🔄 Ciclo de Desarrollo
1. **Red:** Crea un test en el módulo correspondiente (ej. `src/app.rs`, `src/curl.rs`) que defina la nueva funcionalidad. Ejecuta `cargo test` y verifica que falle.
2. **Green:** Escribe el código mínimo necesario para que el test pase.
3. **Refactor:** Limpia el código, optimiza y asegúrate de que el estilo sea coherente.
4. **Verify:** Ejecuta la suite completa de tests para asegurar que no hay regresiones.

## 🚀 Proceso de Publicación (Homebrew)
Solo cuando los tests pasen al 100%, se procede a:
1. Incrementar versión en `Cargo.toml`.
2. `git add . && git commit -m "..."`.
3. `git tag vX.Y.Z && git push origin master --tags`.
4. Actualizar el SHA256 en `homebrew-tap/arthema.rb`.
5. `git push` en el repositorio del Tap.

---
*“Move fast, but don’t break the neural link.”* 🦾

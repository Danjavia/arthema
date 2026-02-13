# 🚀 ARTHEMA ROADMAP

Este documento detalla las próximas funcionalidades y hitos para convertir a **Arthema** en la herramienta CLI definitiva para desarrolladores.

## 📦 Publicación y Distribución
- [ ] **Homebrew Support:** Configurar `homebrew-tap` para permitir `brew install arthema`.
- [ ] **Crates.io:** Publicar el paquete oficial en el registro de Rust para `cargo install arthema`.
- [ ] **Binarios Pre-compilados:** Configurar GitHub Actions para generar binarios para Mac (Intel/Silicon) y Linux.

## 🛠 Funcionalidades Técnicas
- [ ] **Soporte GraphQL:** Añadir un editor dedicado para queries y esquema de GraphQL.
- [ ] **Variables de Entorno:** Gestión de entornos (Dev, Staging, Prod) mediante archivos `.env` o JSON.
- [ ] **Scripts Pre/Post:** Ejecución de lógica personalizada antes o después de una petición (tipo Postman Scripts).
- [ ] **JSON Tree Interactivo:** Motor de plegado/desplegado para objetos anidados en la respuesta.
- [ ] **Exportación de Código:** Generar automáticamente el código del request en Rust (reqwest), JavaScript (fetch/axios) y Python.

## 🧠 Inteligencia Artificial (Gemini 2.5)
- [ ] **Auto-Headers:** Sugerencias automáticas de cabeceras según el endpoint.
- [ ] **Mock Generator:** Crear servidores mock temporales basados en la respuesta analizada por IA.
- [ ] **Test Generator:** Generar pruebas automatizadas sugeridas por Gemini.

## 🎨 UI/UX
- [ ] **Temas Personalizados:** Soporte para diferentes paletas neón (Cyberpunk, Matrix, Synthwave).
- [ ] **Buscador Global:** `Ctrl+F` para buscar texto dentro de respuestas JSON gigantes.
- [ ] **Historial Persistente:** Mejorar la UI del historial con filtros por fecha y éxito/error.

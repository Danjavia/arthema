# ▟████▙ ARTHEMA - Cyberpunk API IDE

Arthema es una alternativa rápida, liviana y futurista a Postman/Bruno, escrita en Rust y potenciada por IA (Gemini 2.5).

## 🎮 Comandos Globales (Modo Navegación)

| Tecla | Acción |
| :--- | :--- |
| `Tab` | Cambiar entre paneles principales (Colecciones, Editor, Respuesta, AI) |
| `Enter` | Ejecutar petición (en Editor) o Cargar ítem (en Colecciones/Historial) |
| `i` | Entrar en modo **Insert** (Edición) en el campo enfocado |
| `f` | Ciclar foco del Editor (**URL** → **Headers** → **Body** → **Attachment**) |
| `m` / `M` | Cambiar método HTTP (GET, POST, etc.) / `M` para retroceder |
| `b` | Ciclar tipo de cuerpo (**JSON**, **TEXT**, **FORM**) |
| `h` | Alternar panel izquierdo entre **Collections** e **History** |
| `n` | Siguiente pestaña de petición |
| `s` | Guardar petición actual en Colecciones |
| `c` | Copiar contenido de la sección activa al portapapeles de Mac |
| `q` | Salir de Arthema |

## 🧠 Comandos de Inteligencia Artificial

| Tecla | Acción |
| :--- | :--- |
| `a` | **AI Suggest:** Sugiere una API según el texto en la URL |
| `e` | **AI Explain:** Analiza y explica la respuesta técnica recibida |
| `x` | **AI Fixer:** Analiza un error de petición y sugiere una corrección |
| `t` | **Tree Mode:** (Roadmap) Alternar vista de árbol para JSON |

## 📝 Comandos de Edición (Modo Insert)

| Tecla | Acción |
| :--- | :--- |
| `Esc` | Volver al modo Navegación |
| `Ctrl + V` | Pegar texto desde macOS |
| `Ctrl + Z` | Deshacer último cambio |
| `Ctrl + A` | Seleccionar todo el texto del campo |
| `Enter` | Ejecutar petición (solo si el foco está en la **URL**) |

## 🖇 Manejo de Archivos (Multipart)

1. Usa `f` hasta llegar al panel de **Attachment**.
2. Presiona `Enter` para abrir el explorador de archivos.
3. Navega con `↑` / `↓`.
4. Selecciona `..` para subir de nivel o una carpeta para entrar.
5. Presiona `Enter` sobre un archivo para adjuntarlo.

## 📡 Roadmap
- [ ] Soporte para **GraphQL**.
- [ ] Variables de entorno dinámicas.
- [ ] Generación automática de código (Rust, JS, Python).
- [ ] Scripts pre y post ejecución.

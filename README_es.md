<p align="center">
  <img src="https://avatars.githubusercontent.com/u/258253854?v=4" alt="PTK - Rust Token Killer" width="500">
</p>

<p align="center">
  <strong>Proxy CLI de alto rendimiento que elimina hasta el 90% de la salida bash que lee tu agente</strong>
</p>

<p align="center">
  <a href="https://github.com/riaku/ptk/actions"><img src="https://github.com/riaku/ptk/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/riaku/ptk/releases"><img src="https://img.shields.io/github/v/release/riaku/ptk" alt="Release"></a>
  <a href="#licencia"><img src="https://img.shields.io/badge/license-Apache--2.0%20%2B%20BUSL--1.1-blue.svg" alt="License: Apache-2.0 and BUSL-1.1"></a>
  <a href="https://discord.gg/RySmvNF5kF"><img src="https://img.shields.io/discord/1478373640461488159?label=Discord&logo=discord" alt="Discord"></a>
</p>

<p align="center">
  <a href="https://www.rtk-ai.app">Sitio web</a> &bull;
  <a href="#instalacion">Instalar</a> &bull;
  <a href="docs/TROUBLESHOOTING.md">Solucion de problemas</a> &bull;
  <a href="docs/contributing/ARCHITECTURE.md">Arquitectura</a> &bull;
  <a href="https://discord.gg/RySmvNF5kF">Discord</a>
</p>

<p align="center">
  <a href="README.md">English</a> &bull;
  <a href="README_fr.md">Francais</a> &bull;
  <a href="README_zh.md">中文</a> &bull;
  <a href="README_ja.md">日本語</a> &bull;
  <a href="README_ko.md">한국어</a> &bull;
  <a href="README_es.md">Espanol</a>
</p>

---

ptk filtra y comprime las salidas de comandos antes de que lleguen al contexto de tu LLM. Binario Rust unico, cero dependencias, <10ms de overhead.

## Que hace PTK

PTK intercepta comandos de shell y comprime su salida antes de que tu agente la lea.

| Operacion | Que hace PTK con la salida |
|-----------|----------------------------|
| `ls` / `tree` | Formato de arbol con conteo de archivos en lugar de una linea por entrada |
| `cat` / `read` | Lectura inteligente: firmas y estructura en vez de cuerpos completos |
| `grep` / `rg` | Trunca lineas largas, agrupa coincidencias por archivo |
| `git status` | Formato stat compacto, agrupado por estado |
| `git diff` | Contexto reducido, cabeceras eliminadas |
| `git log` | Solo hash, autor y asunto |
| `git add/commit/push` | Linea de confirmacion en lugar de la salida de progreso completa |
| `cargo test` / `npm test` | Solo fallos, los tests que pasan se reducen a un contador |
| `ruff check` | Agrupado por regla y archivo |
| `pytest` | Solo fallos, traceback recortado |
| `go test` | NDJSON parseado, solo fallos |
| `docker ps` | Solo campos esenciales |

## Como funciona el ahorro

PTK elimina **hasta el 90% de la salida bash** que lee tu agente. Eso es lo que PTK mide, y no es lo mismo que reducir tu factura en un 90%.

La salida bash es **uno de los factores que alimentan los tokens de entrada**, junto con tu prompt, el prompt del sistema y el historial de conversacion. Los tokens de entrada son a su vez **solo una parte de la factura**, que tambien cuenta los tokens de salida. La reduccion se diluye en cada paso.

Los recuentos de tokens que reporta PTK se estiman como `bytes / 4`: PTK no incluye ningun tokenizador, por lo que los **porcentajes son fiables pero las cifras absolutas de tokens son aproximadas**.

> Explicacion completa: [Como funciona el ahorro en PTK](docs/guide/resources/savings-explained.md)

## Instalacion

### Instalacion rapida (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/riaku/ptk/refs/heads/develop/install.sh | sh
```

### Cargo

```bash
cargo install --git https://github.com/riaku/ptk ptk
```

### Edición Apache-2.0 (solo código abierto, uso comercial permitido)

El mismo binario `ptk` compilado sin el motor de compresión BUSL-1.1: el sistema de reescritura de hooks y todos los filtros de comandos, solo bajo Apache-2.0.

```bash
curl -fsSL https://raw.githubusercontent.com/riaku/ptk/refs/heads/develop/install.sh | PTK_EDITION=oss sh
cargo install --git https://github.com/riaku/ptk ptk --no-default-features
```

### Verificacion

```bash
ptk --version   # Debe mostrar "ptk 0.49.x"
ptk gain        # Debe mostrar estadisticas de ahorro
```

## Inicio rapido

```bash
# 1. Instalar el hook para la herramienta de IA correspondiente
ptk init --global               # Claude Code (predeterminado)
ptk init --agent trae           # Trae (proyecto)
ptk init --global --agent trae  # Trae (global)

# 2. Reiniciar la herramienta de IA correspondiente, luego probar
git status  # Automaticamente reescrito a ptk git status
```

## Como funciona

```
  Sin ptk:                                         Con ptk:

  Claude  --git status-->  shell  -->  git          Claude  --git status-->  PTK  -->  git
    ^                                   |             ^                      |          |
    |        ~2,000 tokens (crudo)      |             |   ~200 tokens        | filtro   |
    +-----------------------------------+             +------- (filtrado) ---+----------+
```

Cuatro estrategias:

1. **Filtrado inteligente** - Elimina ruido (comentarios, espacios, boilerplate)
2. **Agrupacion** - Agrega elementos similares (archivos por directorio, errores por tipo)
3. **Truncamiento** - Mantiene contexto relevante, elimina redundancia
4. **Deduplicacion** - Colapsa lineas de log repetidas con contadores

## Comandos

> Los porcentajes de abajo son **reducciones de bytes de salida bash**, medidas con el estimador `bytes / 4` de PTK. Ver [Como funciona el ahorro](#como-funciona-el-ahorro).

### Archivos
```bash
ptk ls .                        # Arbol de directorios optimizado
ptk read file.rs                # Lectura inteligente
ptk find "*.rs" .               # Resultados compactos
ptk grep "pattern" .            # Busqueda agrupada por archivo
```

### Git
```bash
ptk git status                  # Estado compacto
ptk git log -n 10               # Commits en una linea
ptk git diff                    # Diff condensado
ptk git push                    # -> "ok main"
```

### Tests
```bash
ptk jest                        # Jest compacto
ptk vitest                      # Vitest compacto
ptk pytest                      # Tests Python (-90%)
ptk go test                     # Tests Go (-90%)
ptk cargo test                  # Tests Rust (-90%)
ptk test <cmd> [args...]        # Solo fallos (-90%), argv directo
```

### Build & Lint
```bash
ptk lint                        # ESLint agrupado por regla
ptk tsc                         # Errores TypeScript agrupados
ptk cargo build                 # Build Cargo (-80%)
ptk ruff check                  # Lint Python (-80%)
```

### Analiticas
```bash
ptk gain                        # Estadisticas de ahorro
ptk gain --graph                # Grafico ASCII (30 dias)
ptk discover                    # Descubrir ahorros perdidos
```

## Documentacion

- **[TROUBLESHOOTING.md](docs/guide/resources/troubleshooting.md)** - Resolver problemas comunes
- **[INSTALL.md](INSTALL.md)** - Guia de instalacion detallada
- **[ARCHITECTURE.md](docs/contributing/ARCHITECTURE.md)** - Arquitectura tecnica

## Contribuir

Las contribuciones son bienvenidas. Abre un issue o PR en [GitHub](https://github.com/riaku/ptk).

Unete a la comunidad en [Discord](https://discord.gg/RySmvNF5kF).

## Licencia

Licencia Apache 2.0 - ver [LICENSE](LICENSE) para detalles.

> `engine/` and `proxy/` are licensed BUSL-1.1 — see their LICENSE files.

## Descargo de responsabilidad

Ver [DISCLAIMER.md](DISCLAIMER.md).

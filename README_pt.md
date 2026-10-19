<p align="center">
  <img src="https://avatars.githubusercontent.com/u/258253854?v=4" alt="PTK - Rust Token Killer" width="500">
</p>

<p align="center">
  <strong>Proxy CLI de alta performance que corta até 90% da saída bash que seu agente lê</strong>
</p>

<p align="center">
  <a href="https://github.com/rtk-ai/ptk/actions"><img src="https://github.com/rtk-ai/ptk/workflows/Security%20Check/badge.svg" alt="CI"></a>
  <a href="https://github.com/rtk-ai/ptk/releases"><img src="https://img.shields.io/github/v/release/rtk-ai/ptk" alt="Release"></a>
  <a href="#licença"><img src="https://img.shields.io/badge/license-Apache--2.0%20%2B%20BUSL--1.1-blue.svg" alt="License: Apache-2.0 and BUSL-1.1"></a>
  <a href="https://discord.gg/RySmvNF5kF"><img src="https://img.shields.io/discord/1470188214710046894?label=Discord&logo=discord" alt="Discord"></a>
  <a href="https://formulae.brew.sh/formula/ptk"><img src="https://img.shields.io/homebrew/v/ptk" alt="Homebrew"></a>
</p>

<p align="center">
  <a href="https://www.rtk-ai.app">Site</a> &bull;
  <a href="#instalacao">Instalar</a> &bull;
  <a href="https://www.rtk-ai.app/guide/troubleshooting">Solução de problemas</a> &bull;
  <a href="docs/contributing/ARCHITECTURE.md">Arquitetura</a> &bull;
  <a href="https://discord.gg/RySmvNF5kF">Discord</a>
</p>

<p align="center">
  <a href="README.md">English</a> &bull;
  <a href="README_fr.md">Francais</a> &bull;
  <a href="README_zh.md">中文</a> &bull;
  <a href="README_ja.md">日本語</a> &bull;
  <a href="README_ko.md">한국어</a> &bull;
  <a href="README_es.md">Espanol</a> &bull;
  <a href="README_pt.md">Português</a>
</p>

---

ptk filtra e comprime saídas de comandos antes de chegarem ao contexto do seu LLM. Binário Rust único, zero dependências, overhead inferior a 10ms.

## O que o PTK faz

O PTK intercepta comandos de shell e comprime a saída antes que seu agente a leia.

| Operação | O que o PTK faz com a saída |
|-----------|-----------------------------|
| `ls` / `tree` | Formato de árvore com contagem de arquivos em vez de uma linha por entrada |
| `cat` / `read` | Leitura inteligente: assinaturas e estrutura em vez de corpos completos |
| `grep` / `rg` | Trunca linhas longas, agrupa correspondências por arquivo |
| `git status` | Formato stat compacto, agrupado por estado |
| `git diff` | Contexto reduzido, cabeçalhos removidos |
| `git log` | Apenas hash, autor e assunto |
| `git add/commit/push` | Linha de confirmação em vez da saída de progresso completa |
| `cargo test` / `npm test` | Apenas falhas, testes aprovados reduzidos a um contador |
| `ruff check` | Agrupado por regra e arquivo |
| `pytest` | Apenas falhas, traceback encurtado |
| `go test` | NDJSON parseado, apenas falhas |
| `docker ps` | Apenas campos essenciais |

## Como funciona a economia

O PTK corta **até 90% da saída bash** que seu agente lê. É isso que o PTK mede, e não é a mesma coisa que reduzir sua fatura em 90%.

A saída bash é **um dos contribuintes para os tokens de entrada**, ao lado do seu prompt, do prompt de sistema e do histórico da conversa. Os tokens de entrada são, por sua vez, **apenas parte da fatura**, que também conta os tokens de saída. A redução se dilui a cada etapa.

As contagens de tokens que o PTK reporta são estimadas como `bytes / 4`: o PTK não embarca nenhum tokenizador, portanto os **percentuais são confiáveis, mas os números absolutos de tokens são aproximados**.

> Explicação completa: [Como funciona a economia do PTK](docs/guide/resources/savings-explained.md)

## Instalacao

### Homebrew (recomendado)

```bash
brew install ptk
```

### winget (Windows)

A forma mais simples de instalar no Windows — um único comando, sem configuração de PATH:

```powershell
winget install rtk-ai.ptk
```

### Instalação rápida (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rtk-ai/ptk/refs/heads/master/install.sh | sh
```

### Cargo

```bash
cargo install --git https://github.com/rtk-ai/ptk ptk
```

### Edição Apache-2.0 (apenas código aberto, uso comercial permitido)

O mesmo binário `ptk` compilado sem o motor de compressão BUSL-1.1: o sistema de reescrita de hooks e todos os filtros de comandos, apenas sob Apache-2.0.

```bash
curl -fsSL https://raw.githubusercontent.com/rtk-ai/ptk/refs/heads/master/install.sh | PTK_EDITION=oss sh
brew install rtk-ai/tap/ptk-oss
cargo install --git https://github.com/rtk-ai/ptk ptk --no-default-features
```

### Verificação

```bash
ptk --version   # Deve exibir "ptk 0.28.2"
ptk gain        # Deve exibir estatísticas de economia
```

## Inicio rapido

```bash
# 1. Instalar o hook para a ferramenta de IA correspondente
ptk init --global               # Claude Code (padrão)
ptk init --agent trae           # Trae (projeto)
ptk init --global --agent trae  # Trae (global)

# 2. Reiniciar a ferramenta de IA correspondente, depois testar
git status  # Reescrito automaticamente para ptk git status
```

## Como funciona

```
  Sem ptk:                                        Com ptk:

  Claude  --git status-->  shell  -->  git         Claude  --git status-->  PTK  -->  git
    ^                                   |            ^                      |          |
    |        ~2,000 tokens (bruto)      |            |   ~200 tokens        | filtro   |
    +-----------------------------------+            +------- (filtrado) ---+----------+
```

Quatro estratégias:

1. **Filtragem inteligente** - Elimina ruído (comentários, espaços, boilerplate)
2. **Agrupamento** - Agrega itens similares (arquivos por diretório, erros por tipo)
3. **Truncamento** - Mantém contexto relevante, elimina redundância
4. **Deduplicação** - Colapsa linhas de log repetidas com contadores

## Comandos

> Os percentuais abaixo são **reduções de bytes da saída bash**, medidas com o estimador `bytes / 4` do PTK. Veja [Como funciona a economia](#como-funciona-a-economia).

### Arquivos
```bash
ptk ls .                        # Árvore de diretórios otimizada
ptk read file.rs                # Leitura inteligente
ptk find "*.rs" .               # Resultados compactos
ptk grep "pattern" .            # Busca agrupada por arquivo
```

### Git
```bash
ptk git status                  # Status compacto
ptk git log -n 10               # Commits em uma linha
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
ptk test <cmd> [args...]        # Só falhas (-90%), argv direto
```

### Build & Lint
```bash
ptk lint                        # ESLint agrupado por regra
ptk tsc                         # Erros TypeScript agrupados
ptk cargo build                 # Build Cargo (-80%)
ptk ruff check                  # Lint Python (-80%)
```

### Análises
```bash
ptk gain                        # Estatísticas de economia
ptk gain --graph                # Gráfico ASCII (30 dias)
ptk discover                    # Descobrir economias perdidas
```

## Documentação

- **[INSTALL.md](INSTALL.md)** - Guia de instalação detalhado
- **[ARCHITECTURE.md](docs/contributing/ARCHITECTURE.md)** - Arquitetura técnica
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Guia de contribuição

## Contribuir

Contribuições são bem-vindas. Abra uma issue ou PR no [GitHub](https://github.com/rtk-ai/ptk).

Junte-se à comunidade no [Discord](https://discord.gg/RySmvNF5kF).

## Licença

Apache License 2.0 - veja [LICENSE](LICENSE) para detalhes.

> `engine/` and `proxy/` are licensed BUSL-1.1 — see their LICENSE files.

## Aviso Legal

Veja [DISCLAIMER.md](DISCLAIMER.md).

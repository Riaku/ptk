# PTK - Documentation fonctionnelle complete

> **PTK (Rust Token Killer)** -- Proxy CLI haute performance qui reduit jusqu'a 90% de la sortie bash lue par votre agent.

Binaire Rust unique, zero dependances externes, overhead < 10ms par commande.

## A propos de la reduction de sortie bash

Tous les pourcentages notes **Reduction sortie bash** dans ce document mesurent les **octets de sortie bash supprimes** : la sortie qu'une commande shell renvoie avant que l'agent ne la lise. Ce n'est pas equivalent a une reduction de facture du meme ordre.

La sortie bash n'est qu'une source parmi d'autres pour les tokens d'entree, aux cotes de votre prompt, du prompt systeme et de l'historique de conversation. Les tokens d'entree ne representent eux-memes qu'une partie de la facture, qui compte aussi les tokens de sortie. La reduction se dilue a chaque etape.

Les compteurs de tokens affiches par PTK sont estimes a `octets / 4` : PTK n'embarque pas de tokenizer, donc **les pourcentages sont fiables mais les nombres absolus de tokens sont approximatifs**.

Explication complete : [How PTK Savings Work](../guide/resources/savings-explained.md)

---

## Table des matieres

1. [Vue d'ensemble](#vue-densemble)
2. [Drapeaux globaux](#drapeaux-globaux)
3. [Commandes Fichiers](#commandes-fichiers)
4. [Commandes Git](#commandes-git)
5. [Commandes GitHub CLI](#commandes-github-cli)
6. [Commandes Test](#commandes-test)
7. [Commandes Build et Lint](#commandes-build-et-lint)
8. [Commandes Formatage](#commandes-formatage)
9. [Gestionnaires de paquets](#gestionnaires-de-paquets)
10. [Conteneurs et orchestration](#conteneurs-et-orchestration)
11. [Donnees et reseau](#donnees-et-reseau)
12. [Cloud et bases de donnees](#cloud-et-bases-de-donnees)
13. [Stacked PRs (Graphite)](#stacked-prs-graphite)
14. [Analytique et suivi](#analytique-et-suivi)
15. [Systeme de hooks](#systeme-de-hooks)
16. [Configuration](#configuration)
17. [Systeme Tee (recuperation de sortie)](#systeme-tee)
18. [Telemetrie](#telemetrie)

---

## Vue d'ensemble

PTK agit comme un proxy entre un LLM (Claude Code, Gemini CLI, etc.) et les commandes systeme. Quatre strategies de filtrage sont appliquees selon le type de commande :

| Strategie | Description | Exemple |
|-----------|-------------|---------|
| **Filtrage intelligent** | Supprime le bruit (commentaires, espaces, boilerplate) | `ls -la` -> arbre compact |
| **Regroupement** | Agregation par repertoire, par type d'erreur, par regle | Tests groupes par fichier |
| **Troncature** | Conserve le contexte pertinent, supprime la redondance | Diff condense |
| **Deduplication** | Fusionne les lignes de log repetees avec compteurs | `error x42` |

### Mecanisme de fallback

Si PTK ne reconnait pas une sous-commande, il execute la commande brute (passthrough) et enregistre l'evenement dans la base de suivi. Cela garantit que PTK est **toujours sur** a utiliser -- aucune commande ne sera bloquee.

---

## Drapeaux globaux

Ces drapeaux s'appliquent a **toutes** les sous-commandes :

| Drapeau | Court | Description |
|---------|-------|-------------|
| `--verbose` | `-v` | Augmenter la verbosite (-v, -vv, -vvv). Montre les details de filtrage. |
| `--ultra-compact` | `-u` | Mode ultra-compact : icones ASCII, format inline. Reduit encore la sortie bash. |
| `--skip-env` | -- | Definit `SKIP_ENV_VALIDATION=1` pour les processus enfants (Next.js, tsc, lint, prisma). |

**Exemples :**

```bash
PTK -v git status          # Status compact + details de filtrage sur stderr
PTK -vvv cargo test        # Verbosite maximale (debug)
PTK -u git log             # Log ultra-compact, icones ASCII
PTK --skip-env next build  # Desactive la validation d'env de Next.js
```

---

## Commandes Fichiers

### `PTK ls` -- Listage de repertoire

**Objectif :** Remplace `ls` et `tree` avec une sortie optimisee en tokens.

**Syntaxe :**
```bash
PTK ls [args...]
```

Tous les drapeaux natifs de `ls` sont supportes (`-l`, `-a`, `-h`, `-R`, etc.).

**Avant / Apres :**
```
# ls -la (45 lignes, ~800 tokens)          # PTK ls (12 lignes, ~150 tokens)
drwxr-xr-x  15 user staff 480 ...          my-project/
-rw-r--r--   1 user staff 1234 ...          +-- src/ (8 files)
-rw-r--r--   1 user staff 567 ...           |   +-- main.rs
...40 lignes de plus...                     +-- Cargo.toml
                                            +-- README.md
```

---

### `PTK tree` -- Arbre de repertoire

**Objectif :** Proxy vers `tree` natif avec sortie filtree.

**Syntaxe :**
```bash
PTK tree [args...]
```

Supporte tous les drapeaux natifs de `tree` (`-L`, `-d`, `-a`, etc.).

---

### `PTK read` -- Lecture de fichier

**Objectif :** Remplace `cat`, `head`, `tail` avec un filtrage intelligent du contenu.

**Syntaxe :**
```bash
PTK read <fichier> [options]
PTK read - [options]          # Lecture depuis stdin
```

**Options :**

| Option | Court | Defaut | Description |
|--------|-------|--------|-------------|
| `--level` | `-l` | `minimal` | Niveau de filtrage : `none`, `minimal`, `aggressive` |
| `--max-lines` | `-m` | illimite | Apercu structurel plafonne a N lignes (signatures et imports, pas les N premieres) |
| `--head-lines` | | illimite | Garde seulement les N premieres lignes, a l'octet pres |
| `--tail-lines` | | illimite | Garde seulement les N dernieres lignes, a l'octet pres |
| `--line-numbers` | `-n` | non | Afficher les numeros de ligne |

**Niveaux de filtrage :**

| Niveau | Description | Reduction sortie bash |
|--------|-------------|-----------|
| `none` | Aucun filtrage, sortie brute | 0% |
| `minimal` | Supprime commentaires et lignes vides excessives | ~30% |
| `aggressive` | Signatures uniquement (supprime les corps de fonctions) | ~74% |

**Avant / Apres (mode aggressive) :**
```
# cat main.rs (~200 lignes)                # PTK read main.rs -l aggressive (~50 lignes)
fn main() -> Result<()> {                   fn main() -> Result<()> { ... }
    let config = Config::load()?;           fn process_data(input: &str) -> Vec<u8> { ... }
    let data = process_data(&input);        struct Config { ... }
    for item in data {                      impl Config { fn load() -> Result<Self> { ... } }
        println!("{}", item);
    }
    Ok(())
}
...
```

**Langages supportes pour le filtrage :** Rust, Python, JavaScript, TypeScript, Go, C, C++, Java, Ruby, Shell.

---

### `PTK smart` -- Resume heuristique

**Objectif :** Genere un resume technique de 2 lignes pour un fichier source.

**Syntaxe :**
```bash
PTK smart <fichier> [--model heuristic] [--force-download]
```

**Exemple :**
```
$ PTK smart src/tracking.rs
SQLite-based token tracking system for command executions.
Records input/output tokens, savings %, execution times with 90-day retention.
```

---

### `PTK find` -- Recherche de fichiers

**Objectif :** Remplace `find` et `fd` avec une sortie compacte groupee par repertoire.

**Syntaxe :**
```bash
PTK find [args...]
```

Supporte a la fois la syntaxe PTK et la syntaxe native `find` (`-name`, `-type`, etc.).

**Avant / Apres :**
```
# find . -name "*.rs" (30 lignes)           # PTK find "*.rs" . (8 lignes)
./src/main.rs                                src/ (12 .rs)
./src/git.rs                                   main.rs, git.rs, config.rs
./src/config.rs                                tracking.rs, filter.rs, utils.rs
./src/tracking.rs                              ...6 more
./src/filter.rs                              tests/ (3 .rs)
./src/utils.rs                                 test_git.rs, test_ls.rs, test_filter.rs
...24 lignes de plus...
```

---

### `PTK grep` -- Recherche dans le contenu

**Objectif :** Remplace `grep` et `rg` avec une sortie groupee par fichier, tronquee.

**Syntaxe :**
```bash
PTK grep <pattern> [chemin] [options]
```

**Options :**

| Option | Court | Defaut | Description |
|--------|-------|--------|-------------|
| `--max-len` |  | 80 | Longueur maximale de ligne |
| `--max` |  | 200 | Nombre maximum de resultats |
| `--context-only` |  | non | Afficher uniquement le contexte du match |

Aucune de ces options n'a de raccourci : `-l`, `-m` et `-c` sont reserves aux flags natifs de meme lettre (`--files-with-matches`, `--max-count`, `--count`) et sont transmis au moteur.

Les arguments supplementaires sont transmis au moteur reellement invoque -- `grep` pour `PTK grep`, `rg` pour `PTK rg`, PTK ne substituant jamais l'un a l'autre. Un flag propre a ripgrep (`--glob`, `-t`) n'est donc valide que sous `PTK rg`. Les flags qui changent le format de sortie (`-c`, `-l`, `-L`, `-o`, `-Z`) passent directement au moteur sans filtrage PTK.

Il n'y a pas d'option `--file-type` : `PTK rg -t rust` filtre par type, `PTK grep -t rust` remonte l'erreur de grep (`invalid option -- 't'`).

**Avant / Apres :**
```
# grep -rn "fn run" (20 lignes)             # PTK grep -rn "fn run" (10 lignes)
src/git.rs:45:pub fn run(...)                src/git.rs
src/git.rs:120:fn run_status(...)              45: pub fn run(...)
src/ls.rs:12:pub fn run(...)                   120: fn run_status(...)
src/ls.rs:25:fn run_tree(...)                src/ls.rs
...                                            12: pub fn run(...)
                                               25: fn run_tree(...)
```

---

### `PTK ast-grep` -- Recherche structurelle (AST)

**Objectif :** Remplace `ast-grep` avec une sortie groupee par fichier, plafonnee.

**Syntaxe :**
```bash
PTK ast-grep run -p '<pattern>' [chemin] [options]
```

Regroupe les correspondances par fichier, plafonnees a 5 par fichier et 50 au total ; le surplus est remplace par une note de comptage ("N more match line(s) in X" / "N more file(s) not shown"). ast-grep imprime une ligne par ligne source d'une correspondance, et une correspondance structurelle s'etend sur plusieurs lignes : le decompte porte donc sur les lignes, pas sur les correspondances. Les sorties que PTK ne sait pas decouper (`ast-grep scan`, `--heading`) passent telles quelles. Sur une recherche reelle dans ce depot, ~85% de reduction.

`--json` n'est pas filtre -- une demande explicite de sortie structuree passe telle quelle, sans compression.

---

### `PTK diff` -- Diff condense

**Objectif :** Diff ultra-condense entre deux fichiers (uniquement les lignes modifiees).

**Syntaxe :**
```bash
PTK diff <fichier1> <fichier2>
PTK diff <fichier1>              # Stdin comme second fichier
```

---

### `PTK wc` -- Comptage compact

**Objectif :** Remplace `wc` avec une sortie compacte (supprime les chemins et le padding).

**Syntaxe :**
```bash
PTK wc [args...]
```

Supporte tous les drapeaux natifs de `wc` (`-l`, `-w`, `-c`, etc.).

---

## Commandes Git

### Vue d'ensemble

Toutes les sous-commandes git sont supportees. Les commandes non reconnues sont transmises directement a git (passthrough).

**Options globales git :**

| Option | Description |
|--------|-------------|
| `-C <path>` | Changer de repertoire avant execution |
| `-c <key=value>` | Surcharger une config git |
| `--git-dir <path>` | Chemin vers le repertoire .git |
| `--work-tree <path>` | Chemin vers le working tree |
| `--no-pager` | Desactiver le pager |
| `--no-optional-locks` | Ignorer les locks optionnels |
| `--bare` | Traiter comme repo bare |
| `--literal-pathspecs` | Pathspecs literals |

---

### `PTK git status` -- Status compact

```bash
PTK git status [args...]    # Supporte tous les drapeaux git status
```

**Avant / Apres :**
```
# git status (~20 lignes, ~400 tokens)      # PTK git status (~5 lignes, ~80 tokens)
On branch main                               main | 3M 1? 1A
Your branch is up to date with               M src/main.rs
  'origin/main'.                              M src/git.rs
                                              M tests/test_git.rs
Changes not staged for commit:                ? new_file.txt
  (use "git add <file>..." to update)        A staged_file.rs
  modified:   src/main.rs
  modified:   src/git.rs
  ...
```

---

### `PTK git log` -- Historique compact

```bash
PTK git log [args...]    # Supporte --oneline, --graph, --all, -n, etc.
```

**Avant / Apres :**
```
# git log (50+ lignes)                      # PTK git log -n 5 (5 lignes)
commit abc123def... (HEAD -> main)           abc123 Fix token counting bug
Author: User <user@email.com>               def456 Add vitest support
Date:   Mon Jan 15 10:30:00 2024            789abc Refactor filter engine
                                             012def Update README
    Fix token counting bug                   345ghi Initial commit
...
```

---

### `PTK git diff` -- Diff compact

```bash
PTK git diff [args...]    # Supporte --stat, --cached, --staged, etc.
```

**Avant / Apres :**

`git diff` brut, 21 lignes :
```
diff --git a/git.rs b/git.rs
index 50f7a19..225f918 100644
--- a/git.rs
+++ b/git.rs
@@ -1,3 +1,3 @@
 fn helper(branch: &str) -> String {
-    format!("ko {}", branch)
+    format!("ok {}", branch)
 }
diff --git a/main.rs b/main.rs
index 765936a..7abfb4f 100644
--- a/main.rs
+++ b/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    let x = 42;
-    // old code
+    let config = Config::load()?;
+    config.validate()?;
     println!("start");
 }
```

`PTK git diff`, 24 lignes :
```
git.rs  | 2 +-
 main.rs | 4 ++--
 2 files changed, 3 insertions(+), 3 deletions(-)

Changes:

git.rs
@@ -1,3 +1,3 @@
 fn helper(branch: &str) -> String {
-    format!("ko {}", branch)
+    format!("ok {}", branch)
 }
  +1 -1

main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    let x = 42;
-    // old code
+    let config = Config::load()?;
+    config.validate()?;
     println!("start");
 }
  +2 -2
```

Sur un diff aussi petit, PTK economise peu : 440 octets bruts contre 392
filtres, soit 11 %. La compression vient des diffs ou les en-tetes par fichier
et les plafonds par hunk pesent. Sur un diff de 4 fichiers dont un hunk de 300
lignes changees, la sortie passe de 335 a 140 lignes et de 13 262 a 4 880
octets, soit 63 % de reduction, avec la note de troncature et le rappel
`[full diff: PTK git diff --no-compact]`.

Les lignes de hunk et les en-tetes `@@` sortent en colonne 0, dans la forme
unifiee de git, donc `PTK git diff | grep "^-"` fonctionne. Les annotations
propres a PTK restent indentees de deux espaces pour que ces memes greps ne
les comptent pas : le total par fichier (`  +2 -2`) et la note de troncature
(`  ... (30 deletions, 20 additions truncated)`).

Trois limites de l'audit ancre. Au-dela de 100 lignes de changement par hunk,
`grep -c "^-"` plafonne a ce qui est affiche ; la note de troncature indique
le reste, par signe, mais elle est indentee et donc invisible au meme grep.
Sur un diff combine (`diff --cc`, ce que git produit pour chaque chemin en
conflit pendant un merge, rebase, cherry-pick ou stash pop), le marqueur peut
occuper la deuxieme colonne : ` +ours` est compte dans le total `+N -M` mais
reste invisible a `grep "^+"`. Et la sortie n'est pas un patch applicable :
PTK supprime les en-tetes `diff --git` / `---` / `+++`. Utilisez
`PTK proxy git diff` pour un patch fidele, ou `PTK gh pr diff --patch`, qui
passe sans filtrage.

---

### `PTK git show` -- Show compact

```bash
PTK git show [args...]
```

Affiche le resume du commit + stat + diff compact.

> **Attention (redirection vers un fichier).** Pour un blob volumineux
> (`PTK git show HEAD:gros-fichier`), la sortie est fenetree : seul un apercu
> est affiche, suivi d'un indice `[see remaining: git show 'HEAD:...' | tail -n +N]`.
> Un `PTK git show HEAD:x > fichier` ecrit a la main peut donc tronquer
> silencieusement le contenu (le code de sortie reste 0). Pour capturer le
> fichier complet, utilisez `git show` directement, ou suivez l'indice de
> recuperation.

---

### `PTK git add` -- Add ultra-compact

```bash
PTK git add [args...]    # Supporte -A, -p, --all, etc.
```

**Sortie :** `ok` (un seul mot)

---

### `PTK git commit` -- Commit ultra-compact

```bash
PTK git commit -m "message" [args...]    # Supporte -a, --amend, --allow-empty, etc.
```

**Sortie :** `ok abc1234` (confirmation + hash court)

---

### `PTK git push` -- Push ultra-compact

```bash
PTK git push [args...]    # Supporte -u, remote, branch, etc.
```

**Avant / Apres :**
```
# git push (15 lignes, ~200 tokens)         # PTK git push (1 ligne, ~10 tokens)
Enumerating objects: 5, done.                ok main
Counting objects: 100% (5/5), done.
Delta compression using up to 8 threads
...
```

---

### `PTK git pull` -- Pull ultra-compact

```bash
PTK git pull [args...]
```

**Sortie :** `ok 3 files +10 -2`

---

### `PTK git branch` -- Branches compact

```bash
PTK git branch [args...]    # Supporte -d, -D, -m, etc.
```

Affiche branche courante, branches locales, branches distantes de facon compacte.

---

### `PTK git fetch` -- Fetch compact

```bash
PTK git fetch [args...]
```

**Sortie :** `ok fetched (N new refs)`

---

### `PTK git stash` -- Stash compact

```bash
PTK git stash [list|show|pop|apply|drop|push] [args...]
```

---

### `PTK git worktree` -- Worktree compact

```bash
PTK git worktree [add|remove|prune|list] [args...]
```

---

### Passthrough git

Toute sous-commande git non listee ci-dessus est executee directement :

```bash
PTK git rebase main        # Execute git rebase main
PTK git cherry-pick abc    # Execute git cherry-pick abc
PTK git tag v1.0.0         # Execute git tag v1.0.0
```

---

## Commandes GitHub CLI

### `PTK gh` -- GitHub CLI compact

**Objectif :** Remplace `gh` avec une sortie optimisee.

**Syntaxe :**
```bash
PTK gh <sous-commande> [args...]
```

**Sous-commandes supportees :**

| Commande | Description | Reduction sortie bash |
|----------|-------------|-----------|
| `PTK gh pr list` | Liste des PRs compacte | ~80% |
| `PTK gh pr view <num>` | Details d'une PR + checks | ~87% |
| `PTK gh pr checks` | Status des checks CI | ~79% |
| `PTK gh issue list` | Liste des issues compacte | ~80% |
| `PTK gh run list` | Status des workflow runs | ~82% |
| `PTK gh api <endpoint>` | Reponse API compacte | ~26% |

**Avant / Apres :**
```
# gh pr list (~30 lignes)                   # PTK gh pr list (~10 lignes)
Showing 10 of 15 pull requests in org/repo   #42 feat: add vitest (open, 2d)
                                              #41 fix: git diff crash (open, 3d)
#42  feat: add vitest support                 #40 chore: update deps (merged, 5d)
  user opened about 2 days ago                #39 docs: add guide (merged, 1w)
  ... labels: enhancement
...
```

---

## Commandes Test

### `PTK test` -- Wrapper de tests generique

**Objectif :** Execute n'importe quelle commande de test et affiche uniquement les echecs.

**Syntaxe :**
```bash
PTK test <commande...>
```

**Exemple :**
```bash
PTK test cargo test
PTK test npm test
PTK test bun test
PTK test pytest
```

**Avant / Apres :**
```
# cargo test (200+ lignes en cas d'echec)   # PTK test cargo test (~20 lignes)
running 15 tests                             FAILED: 2/15 tests
test utils::test_parse ... ok                  test_edge_case: assertion failed
test utils::test_format ... ok                 test_overflow: panic at utils.rs:18
test utils::test_edge_case ... FAILED
...150 lignes de backtrace...
```

---

### `PTK err` -- Erreurs/avertissements uniquement

**Objectif :** Execute une commande et ne montre que les erreurs et avertissements.

**Syntaxe :**
```bash
PTK err <commande...>
```

**Exemple :**
```bash
PTK err npm run build
PTK err cargo build
```

---

### `PTK cargo test` -- Tests Rust

```bash
PTK cargo test [args...]
```

N'affiche que les echecs. Supporte tous les arguments de `cargo test`.

---

### `PTK cargo nextest` -- Tests Rust (nextest)

```bash
PTK cargo nextest [run|list|--lib] [args...]
```

Filtre la sortie de `cargo nextest` pour n'afficher que les echecs.

---

### `PTK jest` / `PTK vitest` -- Tests Jest/Vitest

```bash
PTK jest [args...]
PTK vitest [args...]
```

---

### `PTK playwright test` -- Tests E2E Playwright

```bash
PTK playwright [args...]
```

---

### `PTK pytest` -- Tests Python

```bash
PTK pytest [args...]
```

---

### `PTK go test` -- Tests Go

```bash
PTK go test [args...]
```

Utilise le streaming JSON NDJSON de Go pour un filtrage precis.

---

## Commandes Build et Lint

### `PTK cargo build` -- Build Rust

```bash
PTK cargo build [args...]
```

Supprime les lignes "Compiling...", ne conserve que les erreurs et le resultat final.

---

### `PTK cargo check` -- Check Rust

```bash
PTK cargo check [args...]
```

Supprime les lignes "Checking...", ne conserve que les erreurs.

---

### `PTK cargo clippy` -- Clippy Rust

```bash
PTK cargo clippy [args...]
```

Regroupe les avertissements par regle de lint.

---

### `PTK cargo install` -- Install Rust

```bash
PTK cargo install [args...]
```

Supprime la compilation des dependances, ne conserve que le resultat d'installation et les erreurs.

---

### `PTK tsc` -- TypeScript Compiler

```bash
PTK tsc [args...]
```

Regroupe les erreurs TypeScript par fichier et par code d'erreur.

**Avant / Apres :**
```
# tsc --noEmit (50 lignes)                  # PTK tsc (15 lignes)
src/api.ts(12,5): error TS2345: ...          src/api.ts (3 errors)
src/api.ts(15,10): error TS2345: ...           TS2345: Argument type mismatch (x2)
src/api.ts(20,3): error TS7006: ...            TS7006: Parameter implicitly has 'any'
src/utils.ts(5,1): error TS2304: ...         src/utils.ts (1 error)
...                                            TS2304: Cannot find name 'foo'
```

---

### `PTK lint` -- ESLint / Biome

```bash
PTK lint [args...]
PTK lint biome [args...]
```

Regroupe les violations par regle et par fichier. Auto-detecte le linter.

---

### `PTK prettier` -- Verification du formatage

```bash
PTK prettier [args...]    # ex: PTK prettier --check .
```

Affiche uniquement les fichiers necessitant un formatage.

---

### `PTK format` -- Formateur universel

```bash
PTK format [args...]
```

Auto-detecte le formateur du projet (prettier, black, ruff format) et applique un filtre compact.

---

### `PTK next build` -- Build Next.js

```bash
PTK next [args...]
```

Sortie compacte avec metriques de routes.

---

### `PTK ruff` -- Linter/formateur Python

```bash
PTK ruff check [args...]
PTK ruff format --check [args...]
```

Sortie JSON compressee.

---

### `PTK mypy` -- Type checker Python

```bash
PTK mypy [args...]
```

Regroupe les erreurs de type par fichier.

---

### `PTK golangci-lint` -- Linter Go

```bash
PTK golangci-lint run [args...]
```

Sortie JSON compressee.

---

## Commandes Formatage

### `PTK prettier` -- Prettier

```bash
PTK prettier --check .
PTK prettier --write src/
```

---

### `PTK format` -- Detecteur universel

```bash
PTK format [args...]
```

Detecte automatiquement : prettier, black, ruff format, rustfmt. Applique un filtre compact unifie.

---

## Gestionnaires de paquets

### `PTK pnpm` -- pnpm

| Commande | Description | Reduction sortie bash |
|----------|-------------|-----------|
| `PTK pnpm list [-d N]` | Arbre de dependances compact | ~70% |
| `PTK pnpm outdated` | Paquets obsoletes : `pkg: old -> new` | ~80% |
| `PTK pnpm install` | Filtre les barres de progression | ~60% |
| `PTK pnpm build` | Delegue au filtre Next.js | ~87% |
| `PTK pnpm typecheck` | Delegue au filtre tsc | ~83% |

Les sous-commandes non reconnues sont transmises directement a pnpm (passthrough).

---

### `PTK npm` -- npm

```bash
PTK npm [args...]    # ex: PTK npm run build
```

Filtre le boilerplate npm (barres de progression, en-tetes, etc.).

---

### `PTK npx` -- npx avec routage intelligent

```bash
PTK npx [args...]
```

Route intelligemment vers les filtres specialises :
- `PTK npx tsc` -> filtre tsc
- `PTK npx eslint` -> filtre lint
- `PTK npx prisma` -> filtre prisma
- Autres -> passthrough filtre

---

### `PTK pip` -- pip / uv

```bash
PTK pip list              # Liste des paquets (auto-detecte uv)
PTK pip outdated          # Paquets obsoletes
PTK pip install <pkg>     # Installation
```

Auto-detecte `uv` si disponible et l'utilise a la place de `pip`.

---

### `PTK deps` -- Resume des dependances

**Objectif :** Resume compact des dependances du projet.

```bash
PTK deps [chemin]    # Defaut: repertoire courant
```

Auto-detecte : `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `Gemfile`, etc.

---

### `PTK prisma` -- ORM Prisma

| Commande | Description |
|----------|-------------|
| `PTK prisma generate` | Generation du client (supprime l'ASCII art) |
| `PTK prisma migrate dev [--name N]` | Creer et appliquer une migration |
| `PTK prisma migrate status` | Status des migrations |
| `PTK prisma migrate deploy` | Deployer en production |
| `PTK prisma db-push` | Push du schema |

---

## Conteneurs et orchestration

### `PTK docker` -- Docker

| Commande | Description | Reduction sortie bash |
|----------|-------------|-----------|
| `PTK docker ps` | Liste compacte des conteneurs | ~80% |
| `PTK docker images` | Liste compacte des images | ~80% |
| `PTK docker logs <conteneur>` | Logs dedupliques | ~70% |
| `PTK docker compose ps` | Services Compose compacts | ~80% |
| `PTK docker compose logs [service]` | Logs Compose dedupliques | ~70% |
| `PTK docker compose build [service]` | Resume du build | ~60% |

Les sous-commandes non reconnues sont transmises directement (passthrough).

**Avant / Apres :**
```
# docker ps (lignes longues, ~30 tokens/ligne)    # PTK docker ps (~10 tokens/ligne)
CONTAINER ID   IMAGE          COMMAND     ...      web  nginx:1.25 Up 2d (healthy)
abc123def456   nginx:1.25     "/dock..."  ...      db   postgres:16 Up 2d (healthy)
789012345678   postgres:16    "docker..."           redis redis:7 Up 1d
```

---

### `PTK kubectl` -- Kubernetes

| Commande | Description | Options |
|----------|-------------|---------|
| `PTK kubectl pods [-n ns] [-A]` | Liste compacte des pods | Namespace ou tous |
| `PTK kubectl services [-n ns] [-A]` | Liste compacte des services | Namespace ou tous |
| `PTK kubectl logs <pod> [-c container]` | Logs dedupliques | Container specifique |

Les sous-commandes non reconnues sont transmises directement (passthrough).

---

## Donnees et reseau

### `PTK json` -- Structure JSON

**Objectif :** Affiche la structure d'un fichier JSON sans les valeurs.

```bash
PTK json <fichier> [--depth N]    # Defaut: profondeur 5
PTK json -                         # Depuis stdin
```

**Avant / Apres :**
```
# cat package.json (50 lignes)              # PTK json package.json (10 lignes)
{                                            {
  "name": "my-app",                            name: string
  "version": "1.0.0",                         version: string
  "dependencies": {                            dependencies: { 15 keys }
    "react": "^18.2.0",                        devDependencies: { 8 keys }
    "next": "^14.0.0",                         scripts: { 6 keys }
    ...15 dependances...                     }
  },
  ...
}
```

---

### `PTK env` -- Variables d'environnement

```bash
PTK env                    # Toutes les variables (sensibles masquees)
PTK env -f AWS             # Filtrer par nom
PTK env --show-all         # Inclure les valeurs sensibles
```

Les variables sensibles (tokens, secrets, mots de passe) sont masquees par defaut : `AWS_SECRET_ACCESS_KEY=***`.

---

### `PTK log` -- Logs dedupliques

**Objectif :** Filtre et deduplique la sortie de logs.

```bash
PTK log <fichier>     # Depuis un fichier
PTK log               # Depuis stdin (pipe)
```

Les lignes repetees sont fusionnees : `[ERROR] Connection refused (x42)`.

---

### `PTK curl` -- HTTP avec troncature

```bash
PTK curl [args...]
```

Tronque les reponses longues et sauvegarde la sortie complete dans un fichier pour recuperation.

---

### `PTK wget` -- Telechargement compact

```bash
PTK wget <url> [args...]
PTK wget -O - <url>           # Sortie vers stdout
```

Supprime les barres de progression et le bruit.

---

### `PTK summary` -- Resume heuristique

**Objectif :** Execute une commande et genere un resume heuristique de la sortie.

```bash
PTK summary <commande...>
```

Utile pour les commandes longues dont la sortie n'a pas de filtre dedie.

---

### `PTK proxy` -- Passthrough avec suivi

**Objectif :** Execute une commande **sans filtrage** mais enregistre l'utilisation pour le suivi.

```bash
PTK proxy <commande...>
```

Utile pour le debug : comparer la sortie brute avec la sortie filtree.

---

## Cloud et bases de donnees

### `PTK aws` -- AWS CLI

```bash
PTK aws <service> [args...]
```

Force la sortie JSON et compresse le resultat. Supporte tous les services AWS (sts, s3, ec2, ecs, rds, cloudformation, etc.).

---

### `PTK psql` -- PostgreSQL

```bash
PTK psql [args...]
```

Supprime les bordures de tableaux et compresse la sortie.

---

## Stacked PRs (Graphite)

### `PTK gt` -- Graphite

| Commande | Description |
|----------|-------------|
| `PTK gt log` | Stack log compact |
| `PTK gt submit` | Submit compact |
| `PTK gt sync` | Sync compact |
| `PTK gt restack` | Restack compact |
| `PTK gt create` | Create compact |
| `PTK gt branch` | Branch info compact |

Les sous-commandes non reconnues sont transmises directement ou detectees comme passthrough git.

---

## Analytique et suivi

### Systeme de tracking

PTK enregistre chaque execution de commande dans une base SQLite :

- **Emplacement :** `~/.local/share/PTK/tracking.db` (Linux), `~/Library/Application Support/PTK/tracking.db` (macOS)
- **Retention :** 90 jours automatique
- **Metriques :** tokens entree/sortie, pourcentage de reduction de sortie bash, temps d'execution, projet

---

### `PTK gain` -- Statistiques de reduction de sortie bash

```bash
PTK gain                        # Resume global
PTK gain -p                     # Filtre par projet courant
PTK gain --graph                # Graphe ASCII (30 derniers jours)
PTK gain --history              # Historique recent des commandes
PTK gain --daily                # Ventilation jour par jour
PTK gain --weekly               # Ventilation par semaine
PTK gain --monthly              # Ventilation par mois
PTK gain --all                  # Toutes les ventilations
PTK gain --quota -t pro         # Estimation de la sortie bash economisee sur le quota mensuel
PTK gain --failures             # Log des echecs de parsing (commandes en fallback)
PTK gain --format json          # Export JSON (pour dashboards)
PTK gain --format csv           # Export CSV
```

**Options :**

| Option | Court | Description |
|--------|-------|-------------|
| `--project` | `-p` | Filtrer par repertoire courant |
| `--graph` | `-g` | Graphe ASCII des 30 derniers jours |
| `--history` | `-H` | Historique recent des commandes |
| `--quota` | `-q` | Estimation de la sortie bash economisee sur le quota mensuel |
| `--tier` | `-t` | Tier d'abonnement : `pro`, `5x`, `20x` (defaut: `20x`) |
| `--daily` | `-d` | Ventilation quotidienne |
| `--weekly` | `-w` | Ventilation hebdomadaire |
| `--monthly` | `-m` | Ventilation mensuelle |
| `--all` | `-a` | Toutes les ventilations |
| `--format` | `-f` | Format de sortie : `text`, `json`, `csv` |
| `--failures` | `-F` | Affiche les commandes en fallback |

**Exemple de sortie :**
```
$ PTK gain
PTK Token Savings Summary
  Total commands:     1,247
  Total input:        2,341,000 tokens
  Total output:       468,200 tokens
  Total saved:        1,872,800 tokens (80%)
  Avg per command:    1,501 tokens saved

Top commands:
  git status    312x  -82%
  cargo test    156x  -91%
  git diff       98x  -76%
```

---

### `PTK discover` -- Opportunites manquees

**Objectif :** Analyse l'historique Claude Code pour trouver les commandes qui auraient pu etre optimisees par PTK.

```bash
PTK discover                          # Projet courant, 30 derniers jours
PTK discover --all --since 7          # Tous les projets, 7 derniers jours
PTK discover -p /chemin/projet        # Filtrer par projet
PTK discover --limit 20              # Max commandes par section
PTK discover --format json            # Export JSON
```

**Options :**

| Option | Court | Description |
|--------|-------|-------------|
| `--project` | `-p` | Filtrer par chemin de projet |
| `--limit` | `-l` | Max commandes par section (defaut: 15) |
| `--all` | `-a` | Scanner tous les projets |
| `--since` | `-s` | Derniers N jours (defaut: 30) |
| `--format` | `-f` | Format : `text`, `json` |

---

### `PTK learn` -- Apprendre des erreurs

**Objectif :** Analyse l'historique d'erreurs CLI de Claude Code pour detecter les corrections recurrentes.

```bash
PTK learn                             # Projet courant
PTK learn --all --since 7             # Tous les projets
PTK learn --write-rules               # Generer .claude/rules/cli-corrections.md
PTK learn --min-confidence 0.8        # Seuil de confiance (defaut: 0.6)
PTK learn --min-occurrences 3         # Occurrences minimales (defaut: 1)
PTK learn --format json               # Export JSON
```

---

### `PTK cc-economics` -- Analyse economique Claude Code

**Objectif :** Compare les depenses Claude Code (via ccusage) avec la sortie bash economisee par PTK.

```bash
PTK cc-economics                      # Resume
PTK cc-economics --daily              # Ventilation quotidienne
PTK cc-economics --weekly             # Ventilation hebdomadaire
PTK cc-economics --monthly            # Ventilation mensuelle
PTK cc-economics --all                # Toutes les ventilations
PTK cc-economics --format json        # Export JSON
```

---

### `PTK hook-audit` -- Metriques du hook

**Prerequis :** Necessite `RTK_HOOK_AUDIT=1` dans l'environnement.

```bash
PTK hook-audit                        # 7 derniers jours (defaut)
PTK hook-audit --since 30             # 30 derniers jours
PTK hook-audit --since 0              # Tout l'historique
```

---

## Systeme de hooks

### Fonctionnement

Le hook PTK intercepte les commandes Bash dans Claude Code **avant leur execution** et les reecrit automatiquement en equivalent PTK.

**Flux :**
```
Claude Code "git status"
    |
    v
settings.json -> PreToolUse hook
    |
    v
rtk-rewrite.sh (bash)
    |
    v
PTK rewrite "git status"  ->  "PTK git status"
    |
    v
Claude Code execute "PTK git status"
    |
    v
Sortie filtree retournee a Claude (~10 tokens vs ~200)
```

**Points cles :**
- Claude ne voit jamais la recriture -- il recoit simplement une sortie optimisee
- Le hook est un delegateur leger (~50 lignes bash) qui appelle `PTK rewrite`
- Toute la logique de recriture est dans le registre Rust (`src/discover/registry.rs`)
- Les commandes deja prefixees par `PTK` passent sans modification
- Les heredocs (`<<`) ne sont pas modifies
- Les commandes non reconnues passent sans modification

### Installation

```bash
PTK init -g                     # Installation recommandee (hook + @@PTKMD@@)
PTK init -g --auto-patch        # Non-interactif (CI/CD)
PTK init -g --hook-only         # Hook seul, sans @@PTKMD@@
PTK init --show                 # Verifier l'installation
PTK init -g --uninstall         # Desinstaller
```

### Fichiers installes

| Fichier | Description |
|---------|-------------|
| `~/.claude/hooks/rtk-rewrite.sh` | Script hook (delegue a `PTK rewrite`) |
| `~/.claude/@@PTKMD@@` | Instructions minimales pour le LLM |
| `~/.claude/settings.json` | Enregistrement du hook PreToolUse |

### `PTK rewrite` -- Recriture de commande

Commande interne utilisee par le hook. Imprime la commande reecrite sur stdout (exit 0) ou sort avec exit 1 si aucun equivalent PTK n'existe.

```bash
PTK rewrite "git status"           # -> "PTK git status" (exit 0)
PTK rewrite "terraform plan"       # -> (exit 1, pas de recriture)
PTK rewrite "PTK git status"       # -> "PTK git status" (exit 0, inchange)
```

### `PTK verify` -- Verification d'integrite

Verifie l'integrite du hook installe via un controle SHA-256.

```bash
PTK verify
```

### Commandes reecrites automatiquement

| Commande brute | Reecrite en |
|----------------|-------------|
| `git status/diff/log/add/commit/push/pull` | `PTK git ...` |
| `gh pr/issue/run` | `PTK gh ...` |
| `cargo test/build/clippy/check` | `PTK cargo ...` |
| `cat/head/tail <fichier>` | `PTK read <fichier>` |
| `rg/grep <pattern>` | `PTK grep <pattern>` |
| `ast-grep run -p <pattern>` | `PTK ast-grep run -p <pattern>` |
| `ls` | `PTK ls` |
| `tree` | `PTK tree` |
| `wc` | `PTK wc` |
| `jest` | `PTK jest` |
| `vitest` | `PTK vitest` |
| `tsc` | `PTK tsc` |
| `eslint/biome` | `PTK lint` |
| `prettier` | `PTK prettier` |
| `playwright` | `PTK playwright` |
| `prisma` | `PTK prisma` |
| `ruff check/format` | `PTK ruff ...` |
| `pytest` | `PTK pytest` |
| `mypy` | `PTK mypy` |
| `pip list/install` | `PTK pip ...` |
| `go test/build/vet` | `PTK go ...` |
| `golangci-lint` | `PTK golangci-lint` |
| `docker ps/images/logs` | `PTK docker ...` |
| `kubectl get/logs` | `PTK kubectl ...` |
| `curl` | `PTK curl` |
| `pnpm list/outdated` | `PTK pnpm ...` |

### Exclusion de commandes

Pour empecher certaines commandes d'etre reecrites, ajoutez-les dans `config.toml` :

```toml
[hooks]
exclude_commands = ["curl", "playwright"]
```

Le motif est ancre en debut de commande : `"curl"` exclut `curl https://...` mais pas `curl-config`.
Avant la comparaison, PTK retire le wrapper, l'interpreteur ou le chemin, donc `"playwright"` couvre
`playwright test` comme `npx playwright test` ou `pnpm exec playwright test`, et `"pytest"` couvre
aussi `python3 -m pytest tests/`. Les arguments sont conserves : `"^ls$"` exclut `ls` seul sans
englober `ls -la`.

---

## Configuration

### Fichier de configuration

**Emplacement :** `~/.config/PTK/config.toml` (Linux) ou `~/Library/Application Support/PTK/config.toml` (macOS)

**Commandes :**
```bash
PTK config                # Afficher la configuration actuelle
PTK config --create       # Creer le fichier avec les valeurs par defaut
```

### Structure complete

```toml
[tracking]
enabled = true              # Activer/desactiver le suivi
history_days = 90           # Jours de retention (nettoyage automatique)
database_path = "/custom/path/tracking.db"  # Chemin personnalise (optionnel)

[display]
colors = true               # Sortie coloree
emoji = true                # Utiliser les emojis
max_width = 120             # Largeur maximale de sortie

[filters]
ignore_dirs = [".git", "node_modules", "target", "__pycache__", ".venv", "vendor"]
ignore_files = ["*.lock", "*.min.js", "*.min.css"]

[retriever]
mode = "sqlite"             # sqlite (defaut) | tee (fichiers legacy) | disabled
max_entries = 200           # Nombre max d'entrees dans la base recall
retention_days = 30         # Retention des entrees
# database_path = "/custom/recall.db"  # Chemin personnalise (optionnel)
# Une ancienne section [tee] reste reconnue : mappee vers mode = "tee",
# ou "disabled" si enabled = false

[telemetry]
enabled = false             # Telemetrie anonyme (1 ping/jour, requiert consentement)
# consent_given = true      # Defini automatiquement par `PTK init` ou `PTK telemetry enable`
# consent_date = "..."      # Date du consentement (RFC 3339)

[hooks]
exclude_commands = []       # Commandes a exclure de la recriture automatique
```

### Variables d'environnement

| Variable | Description |
|----------|-------------|
| `RTK_RECALL=0` | Desactiver la sauvegarde recall |
| `RTK_TEE_DIR` | Surcharge le repertoire tee (mode "tee") |
| `RTK_TELEMETRY_DISABLED=1` | Desactiver la telemetrie |
| `RTK_HOOK_AUDIT=1` | Activer l'audit du hook |
| `SKIP_ENV_VALIDATION=1` | Desactiver la validation d'env (Next.js, etc.) |

---

## Systeme Recall

### Recuperation de sortie brute

Quand une commande echoue (ou qu'un filtre tronque une liste), PTK sauvegarde la sortie brute complete dans une base SQLite locale, adressee par hash de contenu. Cela permet au LLM de recuperer la sortie sans re-executer la commande.

**Fonctionnement :**
1. La commande echoue (exit code != 0) ou la sortie est tronquee
2. PTK stocke la sortie brute (gzip) dans `~/.local/share/PTK/recall.db`
3. Un hint avec le hash est affiche : `[full output: PTK recall <hash>]` ou `[+N hidden: PTK recall <hash>]`
4. `PTK recall <hash>` restitue la partie manquante (delta), `--full` la sortie complete

**Commandes :**
```bash
PTK recall <hash>            # Partie non montree (delta)
PTK recall <hash> --full     # Sortie complete
PTK recall <hash> --grep RE  # Filtrer par regex
PTK recall --list            # Lister les entrees
PTK gain --recalls           # Taux de consultation par filtre (calibration des caps)
PTK config recall <mode>     # Changer de mode sans editer la config (sqlite|tee|disabled)
```

`PTK gain --recalls` separe strictement les donnees par mode : taux exact en sqlite
(la lecture passe par `PTK recall`), approximation (`≥`) en tee (seules les lectures
bash sont detectables via le hook, pas l'outil Read). Un taux eleve signale un filtre
qui cache des sorties que l'agent revient chercher.

**Configuration :**

| Parametre | Defaut | Description |
|-----------|--------|-------------|
| `retriever.mode` | `"sqlite"` | `"sqlite"`, `"tee"` (fichiers legacy), `"disabled"` |
| `retriever.max_entries` | `200` | Eviction FIFO au-dela |
| `retriever.retention_days` | `30` | Purge des entrees anciennes |
| `retriever.max_entry_bytes` | `10 Mo` | Troncature au-dela (a la derniere ligne complete) |
| Taille min | 500 octets | Les echecs trop courts ne sont pas sauvegardes |

Le mode `"tee"` conserve l'ancien comportement (fichiers `.log` dans `~/.local/share/PTK/tee/`, rotation `tee_max_files`). Une ancienne section `[tee]` en config est automatiquement mappee (voir plus haut).

Le mode se change avec `PTK config recall <sqlite|tee|disabled>` (edition chirurgicale de
config.toml, commentaires preserves, champs `[tee]` legacy migres) — le champ equivalent
en edition manuelle est `[retriever] mode`.

---

## Telemetrie

PTK peut envoyer un ping anonyme une fois par jour (23h d'intervalle) pour des statistiques d'utilisation. La telemetrie est **desactivee par defaut** et requiert un consentement explicite (RGPD Art. 6, 7).

**Donnees envoyees :** hash de device (SHA-256 d'un sel aleatoire), version, OS, architecture, nombre de commandes/24h, top commandes, pourcentage de reduction de sortie bash, compteurs recall par famille de filtre (elisions/consultations — noms issus d'une allowlist fermee, jamais de hash, chemin, argument ou contenu).

**Responsable du traitement :** `the PTK fork maintainers`, the GitHub issue tracker (https://github.com/Riaku/ptk/issues)

**Gerer la telemetrie :**
```bash
PTK telemetry status     # Voir l'etat du consentement
PTK telemetry enable     # Donner son consentement (prompt interactif)
PTK telemetry disable    # Retirer son consentement
PTK telemetry forget     # Retirer + supprimer donnees locales + demande d'effacement serveur
```

**Desactiver via variable d'environnement :**
```bash
export RTK_TELEMETRY_DISABLED=1
```

Aucune donnee personnelle, aucun contenu de commande, aucun chemin de fichier n'est transmis. Conservation serveur : 12 mois max. Details : [docs/TELEMETRY.md](../TELEMETRY.md)

---

## Resume de la reduction de sortie bash par categorie

Octets de sortie bash supprimes (voir [A propos de la reduction de sortie bash](#a-propos-de-la-reduction-de-sortie-bash)).

| Categorie | Commandes | Reduction sortie bash |
|-----------|-----------|-------------------|
| **Fichiers** | ls, tree, read, find, grep, ast-grep, diff | 60-85% |
| **Git** | status, log, diff, show, add, commit, push, pull | 75-92% |
| **GitHub** | pr, issue, run, api | 79-87% |
| **Tests** | cargo test, vitest, playwright, pytest, go test | 90-99% |
| **Build/Lint** | cargo build, tsc, eslint, prettier, next, ruff, clippy | 70-87% |
| **Paquets** | pnpm, npm, pip, deps, prisma | 60-80% |
| **Conteneurs** | docker, kubectl | 70-80% |
| **Donnees** | json, env, log, curl, wget | 60-80% |
| **Analytique** | gain, discover, learn, cc-economics | N/A (meta) |

---

## Nombre total de commandes

PTK supporte **45+ commandes** reparties en 9 categories, avec passthrough automatique pour les sous-commandes non reconnues. Cela en fait un proxy universel : il est toujours sur a utiliser en prefixe.

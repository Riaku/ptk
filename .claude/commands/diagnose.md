---
model: haiku
description: PTK environment diagnostics - Checks installation, hooks, version, command routing
---

# /diagnose

Vérifie l'état de l'environnement PTK et suggère des corrections.

## Quand utiliser

- **Automatiquement suggéré** quand Claude détecte ces patterns d'erreur :
  - `PTK: command not found` → PTK non installé ou pas dans PATH
  - Hook errors in Claude Code → Hooks mal configurés ou non exécutables
  - `Unknown command` dans PTK → Version incompatible ou commande non supportée
  - Token savings reports missing → `PTK gain` not working
  - Command routing errors → Hook integration broken

- **Manuellement** après installation, mise à jour PTK, ou si comportement suspect

## Exécution

### 1. Vérifications parallèles

Lancer ces commandes en parallèle :

```bash
# PTK installation check
which PTK && PTK --version || echo "❌ PTK not found in PATH"
```

```bash
# Git status (verify working directory)
git status --short && git branch --show-current
```

```bash
# Hook configuration check
if [ -f ".claude/hooks/rtk-rewrite.sh" ]; then
    echo "✅ OK: rtk-rewrite.sh hook present"
    # Check if hook is executable
    if [ -x ".claude/hooks/rtk-rewrite.sh" ]; then
        echo "✅ OK: hook is executable"
    else
        echo "⚠️ WARNING: hook not executable (chmod +x needed)"
    fi
else
    echo "❌ MISSING: rtk-rewrite.sh hook"
fi
```

```bash
# Hook PTK-suggest.sh check
if [ -f ".claude/hooks/PTK-suggest.sh" ]; then
    echo "✅ OK: PTK-suggest.sh hook present"
    if [ -x ".claude/hooks/PTK-suggest.sh" ]; then
        echo "✅ OK: hook is executable"
    else
        echo "⚠️ WARNING: hook not executable (chmod +x needed)"
    fi
else
    echo "❌ MISSING: PTK-suggest.sh hook"
fi
```

```bash
# Claude Code context check
if [ -n "$CLAUDE_CODE_HOOK_BASH_TEMPLATE" ]; then
    echo "✅ OK: Running in Claude Code context"
    echo "   Hook env var set: CLAUDE_CODE_HOOK_BASH_TEMPLATE"
else
    echo "⚠️ WARNING: Not running in Claude Code (hooks won't activate)"
    echo "   CLAUDE_CODE_HOOK_BASH_TEMPLATE not set"
fi
```

```bash
# Test command routing (dry-run)
if command -v PTK >/dev/null 2>&1; then
    # Test if PTK gain works (validates install)
    if PTK --help | grep -q "gain"; then
        echo "✅ OK: PTK gain available"
    else
        echo "❌ MISSING: PTK gain command (old version or wrong binary)"
    fi
else
    echo "❌ PTK binary not found"
fi
```

### 2. Validate token analytics

```bash
# Run PTK gain to verify analytics work
if command -v PTK >/dev/null 2>&1; then
    echo ""
    echo "📊 Token Savings (last 5 commands):"
    PTK gain --history 2>&1 | head -8 || echo "⚠️ PTK gain failed"
else
    echo "⚠️ Cannot test PTK gain (binary not installed)"
fi
```

### 3. Quality checks (if in PTK repo)

```bash
# Only run if we're in PTK repository
if [ -f "Cargo.toml" ] && grep -q 'name = "PTK"' Cargo.toml 2>/dev/null; then
    echo ""
    echo "🦀 PTK Repository Quality Checks:"

    # Check if cargo fmt passes
    if cargo fmt --all --check >/dev/null 2>&1; then
        echo "✅ OK: cargo fmt (code formatted)"
    else
        echo "⚠️ WARNING: cargo fmt needed"
    fi

    # Check if cargo clippy would pass (don't run full check, just verify binary)
    if command -v cargo-clippy >/dev/null 2>&1 || cargo clippy --version >/dev/null 2>&1; then
        echo "✅ OK: cargo clippy available"
    else
        echo "⚠️ WARNING: cargo clippy not installed"
    fi
else
    echo "ℹ️ Not in PTK repository (skipping quality checks)"
fi
```

## Format de sortie

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔍 PTK Environment Diagnostic
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📦 PTK Binary:      ✅ OK (v0.16.0) | ❌ NOT FOUND
🔗 Hooks:           ✅ OK (rtk-rewrite.sh + PTK-suggest.sh executable)
                    ❌ MISSING or ⚠️ WARNING (not executable)
📊 Token Analytics: ✅ OK (PTK gain working)
                    ❌ FAILED (command not available)
🎯 Claude Context:  ✅ OK (hook environment detected)
                    ⚠️ WARNING (not in Claude Code)
🦀 Code Quality:    ✅ OK (fmt + clippy ready) [if in PTK repo]
                    ⚠️ WARNING (needs formatting/clippy)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Actions suggérées

Utiliser `AskUserQuestion` si problèmes détectés :

```
question: "Problèmes détectés. Quelles corrections appliquer ?"
header: "Fixes"
multiSelect: true
options:
  - label: "cargo install --path ."
    description: "Installer PTK localement depuis le repo"
  - label: "chmod +x .claude/hooks/bash/*.sh"
    description: "Rendre les hooks exécutables"
  - label: "Tout corriger (recommandé)"
    description: "Install PTK + fix hooks permissions"
```

**Adaptations selon contexte** :

### Si PTK non installé
```
options:
  - label: "cargo install --path ."
    description: "Installer PTK localement (si dans le repo)"
  - label: "cargo install --git https://github.com/Riaku/ptk"
    description: "Installer PTK depuis le repo GitHub (dernière release)"
  - label: "Télécharger un binaire précompilé"
    description: "Depuis https://github.com/Riaku/ptk/releases (Windows/macOS/Linux)"
```

### Si hooks manquants/non exécutables
```
options:
  - label: "chmod +x .claude/hooks/*.sh"
    description: "Rendre tous les hooks exécutables"
  - label: "Copier hooks depuis template"
    description: "Si hooks manquants, copier depuis repository principal"
```

### Si PTK gain échoue
```
options:
  - label: "Réinstaller PTK"
    description: "cargo install --path . --force (version outdated?)"
  - label: "Vérifier version"
    description: "PTK --version (besoin v0.16.0+ pour PTK gain)"
```

## Exécution des fixes

### Fix 1 : Installer PTK localement
```bash
# Depuis la racine du repo PTK
cargo install --path .
# Vérifier installation
which PTK && PTK --version
```

### Fix 2 : Rendre hooks exécutables
```bash
chmod +x .claude/hooks/*.sh
# Vérifier permissions
ls -la .claude/hooks/*.sh
```

### Fix 3 : Tout corriger (recommandé)
```bash
# Install PTK
cargo install --path .

# Fix hooks permissions
chmod +x .claude/hooks/*.sh

# Verify
which PTK && PTK --version && PTK gain --history | head -3
```

## Détection automatique

**IMPORTANT** : Claude doit suggérer `/diagnose` automatiquement quand il voit :

| Erreur | Pattern | Cause probable |
|--------|---------|----------------|
| PTK not found | `PTK: command not found` | Pas installé ou pas dans PATH |
| Hook error | Hook execution failed, permission denied | Hooks non exécutables (`chmod +x` needed) |
| Version mismatch | `Unknown command` in PTK output | Version PTK incompatible (upgrade needed) |
| No analytics | `PTK gain` fails or command not found | PTK install incomplete or old version |
| Command not rewritten | Commands not proxied via PTK | Hook integration broken (check `CLAUDE_CODE_HOOK_BASH_TEMPLATE`) |

### Exemples de suggestion automatique

**Cas 1 : PTK command not found**
```
Cette erreur "PTK: command not found" indique que PTK n'est pas installé
ou pas dans le PATH. Je suggère de lancer `/diagnose` pour vérifier
l'installation et obtenir les commandes de fix.
```

**Cas 2 : Hook permission denied**
```
L'erreur "Permission denied" sur le hook rtk-rewrite.sh indique que
les hooks ne sont pas exécutables. Lance `/diagnose` pour identifier
le problème et corriger les permissions avec `chmod +x`.
```

**Cas 3 : PTK gain unavailable**
```
La commande `PTK gain` échoue, ce qui suggère une version PTK obsolète
ou une installation incomplète. `/diagnose` va vérifier la version et
suggérer une réinstallation si nécessaire.
```

## Troubleshooting Common Issues

### Issue : PTK installed but not in PATH

**Symptom**: `cargo install --path .` succeeds but `which PTK` fails

**Diagnosis**:
```bash
# Check if binary installed in Cargo bin
ls -la ~/.cargo/bin/PTK

# Check if ~/.cargo/bin in PATH
echo $PATH | grep -q .cargo/bin && echo "✅ In PATH" || echo "❌ Not in PATH"
```

**Fix**:
```bash
# Add to ~/.zshrc or ~/.bashrc
export PATH="$HOME/.cargo/bin:$PATH"

# Reload shell
source ~/.zshrc  # or source ~/.bashrc
```

### Issue : Multiple PTK binaries (name collision)

**Symptom**: `PTK gain` fails with "command not found" even though `PTK --version` works

**Diagnosis**:
```bash
# Check if wrong PTK installed (reachingforthejack/PTK)
PTK --version
# Should show "PTK X.Y.Z", NOT "Rust Type Kit"

PTK --help | grep gain
# Should show "gain" command - if missing, wrong binary
```

**Fix**:
```bash
# Uninstall wrong PTK
cargo uninstall PTK

# Install correct PTK (this repo)
cargo install --path .

# Verify
PTK gain --help  # Should work
```

### Issue : Hooks not triggering in Claude Code

**Symptom**: Commands not rewritten to `PTK <cmd>` automatically

**Diagnosis**:
```bash
# Check if in Claude Code context
echo $CLAUDE_CODE_HOOK_BASH_TEMPLATE
# Should print hook template path - if empty, not in Claude Code

# Check hooks exist and executable
ls -la .claude/hooks/*.sh
# Should show -rwxr-xr-x (executable)
```

**Fix**:
```bash
# Make hooks executable
chmod +x .claude/hooks/*.sh

# Verify hooks load in new Claude Code session
# (restart Claude Code session after chmod)
```

## Version Compatibility Matrix

| PTK Version | PTK gain | PTK discover | Python/Go support | Notes |
|-------------|----------|--------------|-------------------|-------|
| v0.14.x     | ❌ No    | ❌ No        | ❌ No             | Outdated, upgrade |
| v0.15.x     | ✅ Yes   | ❌ No        | ❌ No             | Missing discover |
| v0.16.x     | ✅ Yes   | ✅ Yes       | ✅ Yes            | **Recommended** |
| main branch | ✅ Yes   | ✅ Yes       | ✅ Yes            | Latest features |

**Upgrade recommendation**: If running v0.15.x or older, upgrade to v0.16.x:

```bash
# From the PTK repo root
git pull origin main
cargo install --path . --force
PTK --version  # Should show 0.16.x or newer
```

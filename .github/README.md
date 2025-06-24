# Generic Theater Actor Release Template

This directory contains a completely generic GitHub Actions setup for releasing theater actors. You can copy this to any actor repository in your actor-registry.

## 📋 What's Included

- `workflows/release.yml` - Main release workflow
- `actions/release-actor/action.yml` - Reusable action for building and releasing actors

## 🚀 How to Use

1. **Copy the template:**
   ```bash
   cp -r .github-template/* .github/
   ```

2. **Commit and push:**
   ```bash
   git add .github/
   git commit -m "Add GitHub Actions release workflow"
   git push
   ```

3. **Create a release:**
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

## ✨ Features

### Completely Generic
- **Auto-detects actor name** from repository name
- **Dynamic content** adapts to any actor
- **Professional formatting** with emojis and clear structure
- **No hardcoded values** - works for any theater actor

### What Gets Released
- `component.wasm` - Compiled WebAssembly component
- `manifest.toml` - Updated with GitHub release URLs
- `init.json` - Initial state (if present)

### Release Page Features
- Clear installation instructions
- Direct manifest URL for easy copying
- Links back to repository and build logs
- Professional, consistent formatting across all actors

## 🔧 Customization

The template is designed to work out-of-the-box, but you can customize:

- **Release body content** in `workflows/release.yml`
- **Build parameters** in `actions/release-actor/action.yml`
- **File inclusion** by modifying the `files:` section

## 📝 Requirements

Your actor repository should have:
- `Cargo.toml` with actor configuration
- `manifest.toml` with actor manifest
- `init.json` (optional) for initial state
- Standard Rust + WebAssembly component structure

That's it! The template handles everything else automatically.

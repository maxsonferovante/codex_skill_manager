# Codex Skill Manager Desktop

Aplicacao desktop para gerenciar skills do Codex, feita com Tauri + Nuxt.

## Estrutura

- `frontend/` - interface Nuxt
- `src-tauri/` - backend Rust/Tauri
- `public/` - assets estaticos, se necessario

## O que ficou fora

- scripts e codigos Python antigos
- testes e cobertura temporarios
- artefatos gerados de Tauri

## Desenvolvimento

```bash
npm install
npm run dev
```

## Tauri

```bash
npm run tauri:dev
npm run tauri:build
```

## Release

Antes de publicar:

1. validar a interface com `npm run build`
2. validar o app desktop com `npm run tauri:build`
3. revisar o conteudo de `src-tauri/` e `frontend/` para manter somente o que e necessario
4. adicionar screenshots e notas de release no README raiz

## Screenshots

Use imagens exportadas do app em:

- `../../docs/screenshots/manager.png`
- `../../docs/screenshots/create.png`
- `../../docs/screenshots/release.png`

# Codex Skill Manager

Aplicacao desktop para organizar, criar e importar skills do Codex com uma interface grafica em Nuxt e um backend Tauri/Rust.

O objetivo e centralizar o ciclo de vida das skills em uma unica ferramenta:

- listar skills ativas e desativadas
- mover skills entre `skills/` e `skills_disabled/`
- criar uma skill nova a partir de um template
- importar uma skill a partir de um arquivo `.md`
- validar nome, slug, descricao e conteudo antes de salvar
- abrir a pasta de skills no sistema operacional

O aplicativo principal fica em [apps/desktop](/Users/mferovante/Documents/workspace/codex_skill_manager/apps/desktop).

## Para que serve

Use esta ferramenta quando quiser:

- manter o conjunto de skills do Codex organizado
- desativar skills sem apagar o conteudo
- publicar uma skill nova de forma guiada
- revisar um `.md` antes de transformar em skill
- reduzir o atrito de gerenciar varias skills localmente

Em vez de editar arquivos manualmente em varias pastas, o app concentra as operacoes em uma interface unica e valida os dados antes de escrever no disco.

## Como funciona

O projeto segue uma arquitetura desktop com duas partes:

- `frontend/` em Nuxt, responsavel pela interface
- `src-tauri/` em Rust, responsavel pelos comandos nativos e operacoes no sistema de arquivos

Fluxo geral:

1. o frontend carrega a lista de skills do backend
2. o backend identifica quais estao em `skills/` e quais estao em `skills_disabled/`
3. o usuario seleciona uma acao:
   - habilitar
   - desabilitar
   - criar a partir de template
   - importar de Markdown
4. o backend valida, planeja e executa a operacao
5. o frontend acompanha logs, conflitos e resultado final

## Funcionalidades

### Gerenciamento de skills

- lista skills habilitadas e desabilitadas
- filtra por slug
- mostra itens ocultos quando necessario
- seleciona varios itens de uma vez
- habilita ou desabilita em lote
- abre a pasta das skills no sistema

### Criacao de skills

- modo de template guiado
- modo de importacao de `.md`
- slug automatico a partir do nome
- edicao manual do slug
- validacao antes de salvar
- preview do `SKILL.md` antes da criacao

### Fluxos nativos

- eventos de operacao com progresso e log
- cancelamento de operacao em andamento
- resolucao de conflitos de nome
- leitura de arquivo Markdown local

## Estrutura

- `apps/desktop/frontend/` - interface Nuxt
- `apps/desktop/src-tauri/` - backend Rust/Tauri
- `apps/desktop/src-tauri/src/pdf_engine/` - pipeline de PDF e extracao
- `LICENSE.md` - licenca
- `.github/workflows/` - automacao de CI

## Tecnologias

<p>
  <img src="apps/desktop/public/icons/tauri.svg" alt="Tauri" width="20" height="20" /> Tauri 2<br>
  <img src="apps/desktop/public/icons/rust-light.svg" alt="Rust" width="20" height="20" /> Rust<br>
  <img src="apps/desktop/public/icons/nuxtjs.svg" alt="Nuxt" width="20" height="20" /> Nuxt 3<br>
  <img src="apps/desktop/public/icons/vuejs.svg" alt="Vue" width="20" height="20" /> Vue 3<br>
  <img src="apps/desktop/public/icons/typescript.svg" alt="TypeScript" width="20" height="20" /> TypeScript
</p>

## Desenvolvimento local

Instale as dependencias na raiz e rode o frontend do desktop:

```bash
npm install
npm run dev:desktop
```

Para rodar o app completo com a shell Tauri:

```bash
npm run tauri:dev
```

Para gerar a build desktop:

```bash
npm run tauri:build
```

## Estrutura de dados esperada

O app trabalha com duas pastas principais no ambiente do Codex:

- `~/.codex/skills/`
- `~/.codex/skills_disabled/`

Esses caminhos podem variar conforme a configuracao do ambiente, mas a ideia permanece a mesma: skills ativas ficam separadas das desativadas para facilitar manutencao sem apagar conteudo.

## Screenshots

Adicione imagens reais do app depois de fechar a interface.

Sugestao de arquivos:

- `docs/screenshots/manager.png`
- `docs/screenshots/create.png`
- `docs/screenshots/release.png`

## Release

Fluxo recomendado:

1. revisar o conteudo da interface e do backend
2. validar o frontend com `npm --workspace apps/desktop run build`
3. validar o desktop com `npm --workspace apps/desktop run tauri:build`
4. capturar screenshots
5. criar a tag de release
6. publicar no GitHub

## Licenca

MIT. Veja [LICENSE.md](/Users/mferovante/Documents/workspace/codex_skill_manager/LICENSE.md).

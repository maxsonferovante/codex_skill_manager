<template>
  <main class="app-shell">
    <header class="header">
      <div><h1>Codex Skill Manager</h1></div>
      <div class="tabs" role="tablist" aria-label="Sections">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          class="tab"
          :class="{ active: activeTab === tab.id }"
          role="tab"
          :aria-selected="activeTab === tab.id"
          @click="activeTab = tab.id"
        >
          {{ tab.label }}
        </button>
      </div>
    </header>

    <section v-if="error" class="error glass-panel">{{ error }}</section>

    <section v-if="activeTab === 'manager'" class="tab-content">
      <section class="toolbar glass-panel">
        <label class="field">
          <span>Filter</span>
          <input v-model="filter" class="input" placeholder="Filter by slug" />
        </label>
        <label class="check">
          <input v-model="includeHidden" type="checkbox" />
          <span>Show hidden/.system</span>
        </label>
      </section>

      <section class="glass-panel panel">
        <div class="actions">
          <button class="button" :disabled="loading || running" @click="refresh">Refresh</button>
          <button class="button" :disabled="running" @click="runOperation('disable', 'selected')">
            Disable selected ({{ selectedEnabledCount }})
          </button>
          <button class="button" :disabled="running" @click="runOperation('enable', 'selected')">
            Enable selected ({{ selectedDisabledCount }})
          </button>
          <button class="button" :disabled="running" @click="runOperation('disable', 'allVisible')">Disable all visible</button>
          <button class="button" :disabled="running" @click="runOperation('enable', 'allVisible')">Enable all visible</button>
          <button class="button" :disabled="running" @click="openSkillsFolderAction">Open skills folder</button>
          <button class="button danger" :disabled="!running" @click="cancelActiveOperation">Cancel</button>
        </div>
        <p class="muted">Enabled: {{ enabledCount }} | Disabled: {{ disabledCount }}</p>
      </section>

      <section class="lists">
        <article class="glass-panel panel">
          <div class="panel-head">
            <h2>Enabled <span class="muted">Selected: {{ selectedEnabledCount }}</span></h2>
            <div class="actions">
              <button class="button" :disabled="running" @click="selectAllVisible('disable')">Select all visible</button>
              <button class="button" :disabled="running" @click="clearSelection('disable')">Clear selection</button>
            </div>
          </div>
          <div class="skill-items">
            <label v-for="entry in enabled" :key="entry.path" class="skill-item">
              <span class="check">
                <input
                  :checked="isSelected('disable', entry.slug)"
                  type="checkbox"
                  :disabled="running"
                  @change="toggleSelectedFromEvent('disable', entry.slug, $event)"
                />
                <span>{{ entry.slug }}</span>
              </span>
              <span class="badge">Active</span>
            </label>
            <p v-if="enabled.length === 0" class="muted">No active skills found.</p>
          </div>
        </article>
        <article class="glass-panel panel">
          <div class="panel-head">
            <h2>Disabled <span class="muted">Selected: {{ selectedDisabledCount }}</span></h2>
            <div class="actions">
              <button class="button" :disabled="running" @click="selectAllVisible('enable')">Select all visible</button>
              <button class="button" :disabled="running" @click="clearSelection('enable')">Clear selection</button>
            </div>
          </div>
          <div class="skill-items">
            <label v-for="entry in disabled" :key="entry.path" class="skill-item">
              <span class="check">
                <input
                  :checked="isSelected('enable', entry.slug)"
                  type="checkbox"
                  :disabled="running"
                  @change="toggleSelectedFromEvent('enable', entry.slug, $event)"
                />
                <span>{{ entry.slug }}</span>
              </span>
              <span class="badge disabled">Disabled</span>
            </label>
            <p v-if="disabled.length === 0" class="muted">No disabled skills found.</p>
          </div>
        </article>
      </section>
    </section>

    <section v-if="activeTab === 'create'" class="tab-content">
      <section class="glass-panel creation-hero">
        <div class="creation-copy">
          <p class="eyebrow">Skill authoring</p>
          <h2>{{ creationMode === 'template' ? 'Criar skill do zero' : 'Refinar skill importada' }}</h2>
          <p class="muted">
            Escreva a skill em uma unica tela. Se quiser partir de um arquivo existente, importe um `.md` e ajuste o conteudo antes de salvar.
          </p>
        </div>
        <div class="actions">
          <button class="button" :class="{ selected: creationMode === 'template' }" @click="switchToTemplateMode">
            New template
          </button>
          <button class="button" :class="{ selected: creationMode === 'import-md' }" :disabled="creationRunning" @click="pickImportMarkdownFileAction">
            Import .md
          </button>
          <button class="button" :disabled="creationRunning" @click="runValidation">
            Validate
          </button>
          <button class="button strong" :disabled="saveDisabled" @click="saveCurrentSkill">
            {{ creationMode === 'template' ? 'Create skill' : 'Save imported skill' }}
          </button>
        </div>
      </section>

      <section class="creation-layout">
        <article class="glass-panel panel">
          <div class="panel-head">
            <h2>Skill data</h2>
            <p class="muted">{{ sourceLabel }}</p>
          </div>
          <div class="toolbar compact">
            <label class="field">
              <span>Skill name</span>
              <input
                v-if="creationMode === 'template'"
                v-model="templateName"
                class="input"
                placeholder="My Skill Name"
              />
              <input
                v-else
                v-model="importName"
                class="input"
                placeholder="Imported Skill Name"
              />
            </label>
            <label class="field">
              <span>Slug</span>
              <input
                v-if="creationMode === 'template'"
                :value="templateSlug"
                class="input"
                @input="onTemplateSlugInput"
              />
              <input
                v-else
                :value="importSlug"
                class="input"
                @input="onImportSlugInput"
              />
            </label>
          </div>

          <label v-if="creationMode === 'template'" class="field">
            <span>Short description</span>
            <input v-model="templateDescription" class="input" placeholder="What this skill does" />
          </label>

          <label v-else class="field">
            <span>Imported file</span>
            <input class="input" :value="importPath || 'No file selected'" readonly />
          </label>

          <label class="field editor-field">
            <span class="field-title">{{ creationMode === 'template' ? 'Main Instructions' : 'Markdown Content' }}</span>
            <span class="field-hint">
              {{ creationMode === 'template' ? 'Write the core guidance, workflow, examples and pitfalls of the skill.' : 'Review and refine the imported markdown before saving.' }}
            </span>
            <textarea
              v-if="creationMode === 'template'"
              v-model="templateInstructions"
              class="input editor"
              placeholder="Describe the skill, guidance, examples and pitfalls."
            />
            <textarea
              v-else
              v-model="importContent"
              class="input editor"
              placeholder="Imported markdown content"
            />
          </label>
        </article>

        <article class="glass-panel panel preview-panel">
          <div class="panel-head">
            <h2>Preview SKILL.md</h2>
            <span class="badge subtle">{{ creationMode === 'template' ? 'Template' : 'Imported' }}</span>
          </div>
          <pre class="preview-block">{{ skillPreview }}</pre>
        </article>
      </section>

      <section v-if="currentIssues.length" class="glass-panel panel">
        <div class="panel-head">
          <h2>Validation</h2>
          <p class="muted">{{ validationSummary }}</p>
        </div>
        <ul class="issues-list">
          <li v-for="issue in currentIssues" :key="`${issue.level}-${issue.code}-${issue.message}`" :class="`issue-${issue.level}`">
            {{ issue.level.toUpperCase() }}: {{ issue.message }}
          </li>
        </ul>
      </section>
    </section>

    <aside v-if="creationStatusVisible && creationLogs.length" class="floating-status glass-panel">
      <div class="panel-head">
        <div>
          <h2>Creation status</h2>
          <p class="muted">{{ creationSummary || latestCreationLog }}</p>
        </div>
        <button class="ghost-button" @click="clearCreationStatus">Close</button>
      </div>
      <ul class="log-list compact-log">
        <li v-for="(line, idx) in creationLogs.slice(-8)" :key="`floating-log-${idx}`">{{ line }}</li>
      </ul>
    </aside>

    <section v-if="pendingConflict" class="modal-backdrop" @click.self="submitConflictResolution('skip')">
      <article class="modal glass-panel">
        <h2>Name conflict</h2>
        <p>Destination already contains '{{ pendingConflict.slug }}'. Overwriting can permanently delete destination contents.</p>
        <label class="check">
          <input v-model="applyToAll" type="checkbox" />
          <span>Apply this choice to all conflicts in this operation</span>
        </label>
        <label class="field">
          <span>Type slug to confirm overwrite</span>
          <input v-model="overwriteConfirmation" class="input" :placeholder="pendingConflict.slug" />
        </label>
        <p v-if="conflictError" class="conflict-error">{{ conflictError }}</p>
        <div class="actions">
          <button class="button" @click="submitConflictResolution('rename')">Rename</button>
          <button class="button danger" :disabled="!canConfirmOverwrite" @click="submitConflictResolution('overwrite')">Overwrite</button>
          <button class="button" @click="submitConflictResolution('skip')">Skip</button>
        </div>
      </article>
    </section>
  </main>
</template>

<script setup lang="ts">
import type { ValidationIssue } from '~/types/contracts';

type TabId = 'manager' | 'create';
const tabs: Array<{ id: TabId; label: string }> = [
  { id: 'manager', label: 'Gestao de Skills' },
  { id: 'create', label: 'Criar Skill' }
];
const activeTab = ref<TabId>('manager');

const {
  enabled,
  disabled,
  enabledCount,
  disabledCount,
  selectedEnabledCount,
  selectedDisabledCount,
  filter,
  includeHidden,
  loading,
  error,
  running,
  pendingConflict,
  overwriteConfirmation,
  conflictError,
  applyToAll,
  canConfirmOverwrite,
  refresh,
  runOperation,
  cancelActiveOperation,
  submitConflictResolution,
  toggleSelectedFromEvent,
  isSelected,
  openSkillsFolderAction,
  selectAllVisible,
  clearSelection,
  creationMode,
  creationLogs,
  creationSummary,
  creationRunning,
  creationStatusVisible,
  templateName,
  templateDescription,
  templateInstructions,
  templateSlug,
  templateValidation,
  validateTemplateAction,
  createTemplateSkillAction,
  setTemplateSlug,
  switchToTemplateMode,
  importPath,
  importContent,
  importName,
  importSlug,
  importValidation,
  pickImportMarkdownFileAction,
  validateImportAction,
  importMarkdownSkillAction,
  setImportSlug,
  clearCreationStatus
} = useSkillsManager();

const sourceLabel = computed(() =>
  creationMode.value === 'template' ? 'Write from a guided template.' : 'Imported markdown can be refined before saving.'
);

const currentIssues = computed<ValidationIssue[]>(() =>
  creationMode.value === 'template' ? templateValidation.value.issues : importValidation.value.issues
);

const saveDisabled = computed(() =>
  creationRunning.value ||
  !(creationMode.value === 'template' ? templateValidation.value.canSubmit : importValidation.value.canSubmit)
);

const latestCreationLog = computed(() => creationLogs.value.at(-1) ?? '');

const validationSummary = computed(() => {
  const errors = currentIssues.value.filter((issue) => issue.level === 'error').length;
  const warnings = currentIssues.value.filter((issue) => issue.level === 'warning').length;
  const infos = currentIssues.value.filter((issue) => issue.level === 'info').length;
  return `Errors: ${errors} | Warnings: ${warnings} | Info: ${infos}`;
});

const skillPreview = computed(() => {
  if (creationMode.value === 'template') {
    return [
      '---',
      `name: ${templateSlug.value}`,
      `description: ${templateDescription.value || 'Short description'}`,
      'type: component',
      '---',
      '',
      `# ${templateName.value || 'Skill Name'}`,
      '',
      '## Purpose',
      templateDescription.value || 'Explain what this skill does.',
      '',
      '## Instructions',
      templateInstructions.value || 'Describe the operating guidance, examples and pitfalls.'
    ].join('\n');
  }

  return importContent.value || '# Imported skill preview';
});

async function runValidation() {
  if (creationMode.value === 'template') {
    await validateTemplateAction();
    return;
  }
  await validateImportAction();
}

async function saveCurrentSkill() {
  if (creationMode.value === 'template') {
    await createTemplateSkillAction();
    return;
  }
  await importMarkdownSkillAction();
}

watch([templateName, templateSlug, templateDescription, templateInstructions], () => {
  if (creationMode.value === 'template') {
    void validateTemplateAction();
  }
});

watch([importName, importSlug, importContent], () => {
  if (creationMode.value === 'import-md') {
    void validateImportAction();
  }
});

function onTemplateSlugInput(event: Event) {
  const target = event.target as HTMLInputElement | null;
  setTemplateSlug(target?.value ?? '');
}

function onImportSlugInput(event: Event) {
  const target = event.target as HTMLInputElement | null;
  setImportSlug(target?.value ?? '');
}

await refresh();
</script>

<style scoped>
.app-shell { display: grid; gap: 16px; max-width: 1180px; margin: 0 auto; padding: 24px; }
.header { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap; }
.tabs { display: flex; gap: 8px; flex-wrap: wrap; }
.tab { border: 1px solid var(--color-border); color: var(--color-text-secondary); background: rgba(125, 211, 252, 0.06); border-radius: 999px; padding: 8px 14px; cursor: pointer; }
.tab.active, .button.selected { color: var(--color-accent); background: rgba(125, 211, 252, 0.16); }
h1 { margin: 0; font-size: 2rem; }
h2 { margin: 0 0 12px; font-size: 1.1rem; }
.eyebrow { margin: 0 0 8px; font-size: 0.76rem; letter-spacing: 0.12em; text-transform: uppercase; color: #7dd3fc; }
.muted { margin: 6px 0 0; color: var(--color-text-secondary); }
.toolbar { display: flex; align-items: flex-end; gap: 16px; padding: 16px; flex-wrap: wrap; }
.toolbar.compact { padding: 0; margin-bottom: 16px; }
.tab-content { display: grid; gap: 16px; }
.field { display: grid; gap: 8px; flex: 1; }
.editor-field { align-content: start; }
.field-title { font-size: 0.95rem; font-weight: 600; letter-spacing: 0.01em; }
.field-hint { font-size: 0.8rem; color: var(--color-text-secondary); line-height: 1.35; }
.check { display: flex; align-items: center; gap: 8px; }
.input { border: 1px solid var(--color-border); background: rgba(17, 24, 39, 0.6); color: var(--color-text-primary); border-radius: 10px; padding: 10px 12px; width: 100%; box-sizing: border-box; min-height: 44px; line-height: 1.2; }
.editor { height: 420px; min-height: 420px; resize: vertical; font-family: ui-monospace, Menlo, monospace; }
.button { border: 1px solid var(--color-border); color: var(--color-accent); background: rgba(125, 211, 252, 0.1); border-radius: 999px; padding: 8px 16px; cursor: pointer; }
.button:disabled { opacity: 0.5; cursor: not-allowed; }
.button.strong { color: #082f49; background: linear-gradient(135deg, #7dd3fc, #fde68a); border-color: rgba(253, 230, 138, 0.45); }
.button.danger { color: #fecaca; border-color: rgba(239, 68, 68, 0.45); background: rgba(239, 68, 68, 0.14); }
.ghost-button { border: 0; background: transparent; color: #7dd3fc; cursor: pointer; }
.lists { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; align-items: stretch; }
.panel-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 12px; flex-wrap: wrap; }
.panel { padding: 16px; }
.skill-items { display: grid; gap: 8px; overflow-y: auto; min-height: 0; padding-right: 4px; }
.skill-item { display: flex; justify-content: space-between; align-items: center; gap: 10px; border: 1px solid var(--color-border); background: rgba(15, 23, 42, 0.5); border-radius: 12px; padding: 8px 10px; }
.badge { font-size: 0.7rem; padding: 3px 8px; border-radius: 999px; color: #bbf7d0; border: 1px solid rgba(34, 197, 94, 0.35); background: rgba(34, 197, 94, 0.14); }
.badge.disabled { color: #fde68a; border-color: rgba(245, 158, 11, 0.35); background: rgba(245, 158, 11, 0.14); }
.badge.subtle { color: #bfdbfe; border-color: rgba(125, 211, 252, 0.25); background: rgba(125, 211, 252, 0.08); }
ul { margin: 0; padding-left: 18px; }
.error { color: #fecaca; border-color: rgba(239, 68, 68, 0.45); padding: 12px; }
.actions { display: flex; gap: 8px; flex-wrap: wrap; }
.creation-hero { display: grid; gap: 16px; grid-template-columns: minmax(0, 1.3fr) minmax(320px, 0.9fr); align-items: end; padding: 20px; }
.creation-copy { display: grid; gap: 6px; }
.creation-layout { --creation-panel-height: 640px; display: grid; gap: 16px; grid-template-columns: minmax(0, 1.2fr) minmax(320px, 0.8fr); align-items: stretch; }
.creation-layout > .panel { height: var(--creation-panel-height); min-height: var(--creation-panel-height); overflow: hidden; }
.preview-panel { display: flex; flex-direction: column; min-height: 0; }
.preview-block { margin: 0; flex: 1 1 auto; min-height: 0; overflow: auto; white-space: pre-wrap; word-break: break-word; border: 1px solid var(--color-border); border-radius: 14px; padding: 16px; background: rgba(15, 23, 42, 0.52); font-family: ui-monospace, Menlo, monospace; line-height: 1.45; }
.issues-list { display: grid; gap: 8px; }
.log-list { max-height: 180px; overflow-y: auto; padding-right: 4px; }
.compact-log { max-height: 220px; }
.floating-status { position: fixed; right: 24px; bottom: 24px; width: min(420px, calc(100vw - 32px)); z-index: 25; padding: 16px; box-shadow: 0 24px 70px rgba(2, 6, 23, 0.38); }
.modal-backdrop { position: fixed; inset: 0; display: grid; place-items: center; background: rgba(2, 6, 23, 0.7); z-index: 30; }
.modal { width: min(560px, calc(100% - 32px)); padding: 16px; }
.conflict-error, .issue-error { color: #fca5a5; }
.issue-warning { color: #fde68a; }
.issue-info { color: #93c5fd; }
@media (max-width: 960px) {
  .lists, .creation-hero, .creation-layout { grid-template-columns: 1fr; }
  .preview-panel { position: static; }
  .floating-status { right: 16px; left: 16px; width: auto; }
}
</style>

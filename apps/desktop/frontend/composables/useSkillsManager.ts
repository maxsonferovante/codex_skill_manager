import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';

import type {
  ConflictAction,
  OperationConflictRequiredEvent,
  PdfExtractionMode,
  RootsInfo,
  SkillEntry,
  StartOperationInput,
  ValidateImportMarkdownOutput,
  ValidateTemplateOutput
} from '~/types/contracts';
import {
  cancelOperation,
  cancelPdfConversion,
  createSkillFromTemplate,
  getRoots,
  importSkillMarkdown,
  isDesktopRuntime,
  listSkills,
  loadMarkdownFile,
  openSkillsFolder,
  pickMarkdownFile,
  pickPdfFiles,
  resolveConflict,
  startPdfConversion,
  startOperation,
  subscribePdfEvents,
  subscribeOperationEvents,
  validateImportMarkdown,
  validateTemplate
} from '~/services/tauri';

function localSlugify(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/(^-|-$)/g, '') || 'skill';
}

export function useSkillsManager() {
  const storageFilterKey = 'skill-manager.filter';
  const storageHiddenKey = 'skill-manager.includeHidden';
  const roots = ref<RootsInfo | null>(null);
  const enabled = ref<SkillEntry[]>([]);
  const disabled = ref<SkillEntry[]>([]);
  const filter = ref('');
  const includeHidden = ref(false);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const logs = ref<string[]>([]);
  const activeOperationId = ref<string | null>(null);
  const running = ref(false);
  const pendingConflict = ref<OperationConflictRequiredEvent | null>(null);
  const overwriteConfirmation = ref('');
  const conflictError = ref<string | null>(null);
  const applyToAll = ref(false);
  const selectedEnabled = ref<Set<string>>(new Set());
  const selectedDisabled = ref<Set<string>>(new Set());
  const pdfDirectory = ref('');
  const pdfFiles = ref<string[]>([]);
  const selectedPdfFiles = ref<Set<string>>(new Set());
  const pdfMode = ref<PdfExtractionMode>('technical');
  const pdfOverwrite = ref(false);
  const pdfRunning = ref(false);
  const pdfOperationId = ref<string | null>(null);
  const pdfProgress = ref('');
  const pdfLogs = ref<string[]>([]);

  const creationMode = ref<'template' | 'import-md'>('template');
  const creationLogs = ref<string[]>([]);
  const creationRunning = ref(false);
  const activeCreationOperationId = ref<string | null>(null);
  const creationStatusVisible = ref(false);

  const templateName = ref('');
  const templateDescription = ref('');
  const templateInstructions = ref('');
  const templateSlug = ref('skill');
  const templateSlugManuallyEdited = ref(false);
  const templateValidation = ref<ValidateTemplateOutput>({ issues: [], canSubmit: false });

  const importPath = ref('');
  const importContent = ref('');
  const importName = ref('');
  const importSlug = ref('skill');
  const importSlugManuallyEdited = ref(false);
  const importValidation = ref<ValidateImportMarkdownOutput>({ issues: [], canSubmit: false });

  const enabledCount = computed(() => enabled.value.length);
  const disabledCount = computed(() => disabled.value.length);
  const selectedEnabledCount = computed(() => selectedEnabled.value.size);
  const selectedDisabledCount = computed(() => selectedDisabled.value.size);
  const creationSummary = computed(() => {
    const last = [...creationLogs.value].reverse().find((line) => /Done:|Finished:|Cancelled after/.test(line));
    return last ?? '';
  });

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      if (!roots.value) {
        roots.value = await getRoots();
      }
      const data = await listSkills({
        includeHidden: includeHidden.value,
        filter: filter.value.trim() || undefined
      });
      enabled.value = data.enabled;
      disabled.value = data.disabled;
      selectedEnabled.value = new Set(
        [...selectedEnabled.value].filter((slug) => data.enabled.some((entry) => entry.slug === slug))
      );
      selectedDisabled.value = new Set(
        [...selectedDisabled.value].filter((slug) => data.disabled.some((entry) => entry.slug === slug))
      );
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
    } finally {
      loading.value = false;
    }
  }

  watch([filter, includeHidden], () => {
    if (typeof window !== 'undefined') {
      window.localStorage?.setItem?.(storageFilterKey, filter.value);
      window.localStorage?.setItem?.(storageHiddenKey, String(includeHidden.value));
    }
    refresh();
  });

  watch(templateName, (value) => {
    if (!templateSlugManuallyEdited.value) {
      templateSlug.value = localSlugify(value);
    }
  });

  watch(importName, (value) => {
    if (!importSlugManuallyEdited.value) {
      importSlug.value = localSlugify(value);
    }
  });

  function log(message: string) {
    logs.value.push(message);
  }

  function logPdf(message: string) {
    pdfLogs.value.push(message);
  }

  function logCreation(message: string) {
    creationLogs.value.push(message);
    creationStatusVisible.value = true;
  }

  async function runOperation(direction: 'enable' | 'disable', mode: 'allVisible' | 'selected') {
    if (running.value || creationRunning.value || pdfRunning.value) return;
    error.value = null;
    const slugs =
      mode === 'selected'
        ? direction === 'disable'
          ? [...selectedEnabled.value]
          : [...selectedDisabled.value]
        : undefined;
    if (mode === 'selected' && (!slugs || slugs.length === 0)) {
      log('Nothing to do.');
      return;
    }
    const input: StartOperationInput = {
      direction,
      mode,
      slugs,
      includeHidden: includeHidden.value,
      filter: filter.value.trim() || undefined
    };
    try {
      running.value = true;
      const result = await startOperation(input);
      activeOperationId.value = result.operationId;
      log(`Planned ${result.plannedTotal} item(s).`);
    } catch (err) {
      running.value = false;
      error.value = err instanceof Error ? err.message : String(err);
    }
  }

  async function cancelActiveOperation() {
    const targetOperationId = activeOperationId.value || activeCreationOperationId.value;
    if (!targetOperationId) return;
    await cancelOperation({ operationId: targetOperationId });
    log('Cancel requested...');
    logCreation('Cancel requested...');
  }

  async function submitConflictResolution(action: ConflictAction) {
    if (!pendingConflict.value) return;
    const current = pendingConflict.value;
    const overwriteConfirmationNormalized = overwriteConfirmation.value.trim();
    if (action === 'overwrite' && overwriteConfirmationNormalized !== current.slug) {
      conflictError.value = `Type '${current.slug}' exactly to confirm overwrite.`;
      return;
    }
    const response = await resolveConflict({
      operationId: current.operationId,
      slug: current.slug,
      action,
      applyToAll: applyToAll.value,
      overwriteConfirmationSlug: action === 'overwrite' ? overwriteConfirmationNormalized : undefined
    });
    if (!response.accepted) {
      conflictError.value = 'Conflict resolution was rejected by backend.';
      return;
    }
    pendingConflict.value = null;
    overwriteConfirmation.value = '';
    conflictError.value = null;
    applyToAll.value = false;
  }

  async function openSkillsFolderAction() {
    try {
      const result = await openSkillsFolder();
      if (result.opened) {
        log(`Opened: ${result.path}`);
      } else {
        log(`Could not open automatically. Skills folder: ${result.path}`);
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
    }
  }

  async function selectPdfFiles() {
    error.value = null;
    try {
      const result = await pickPdfFiles();
      if (!result.paths.length) return;
      const normalized = result.paths.map((path) => path.replace(/\\/g, '/'));
      const directories = new Set(
        normalized.map((path) => {
          const slashIndex = path.lastIndexOf('/');
          return slashIndex > 0 ? path.slice(0, slashIndex) : '';
        })
      );
      if (directories.size !== 1) {
        error.value = 'Selecione PDFs da mesma pasta para esta versão.';
        return;
      }
      const [directory] = [...directories];
      pdfDirectory.value = directory;
      pdfFiles.value = normalized;
      selectedPdfFiles.value = new Set(normalized);
      logPdf(`Selected ${normalized.length} PDF file(s) from ${directory}.`);
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
    }
  }

  async function startPdfConversionAction() {
    if (pdfRunning.value || running.value || creationRunning.value) return;
    if (!pdfDirectory.value.trim()) {
      error.value = 'Selecione os arquivos PDF primeiro.';
      return;
    }
    error.value = null;
    try {
      const selected = [...selectedPdfFiles.value];
      const result = await startPdfConversion({
        directory: pdfDirectory.value.trim(),
        selectedFiles: selected.length ? selected : undefined,
        mode: pdfMode.value,
        overwrite: pdfOverwrite.value
      });
      pdfOperationId.value = result.operationId;
      pdfRunning.value = true;
      pdfProgress.value = `Planned ${result.plannedTotal} file(s).`;
      logPdf(pdfProgress.value);
      logCreation(pdfProgress.value);
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
    }
  }

  async function cancelPdfConversionAction() {
    if (!pdfOperationId.value) return;
    const response = await cancelPdfConversion({ operationId: pdfOperationId.value });
    if (response.accepted) {
      logPdf('Cancel requested...');
      logCreation('Cancel requested...');
    }
  }

  async function validateTemplateAction() {
    templateValidation.value = await validateTemplate({
      name: templateName.value,
      slug: templateSlug.value,
      description: templateDescription.value,
      instructions: templateInstructions.value
    });
    return templateValidation.value;
  }

  async function createTemplateSkillAction() {
    const result = await validateTemplateAction();
    if (!result.canSubmit || creationRunning.value || running.value || pdfRunning.value) return;
    const output = await createSkillFromTemplate({
      name: templateName.value,
      slug: templateSlug.value,
      description: templateDescription.value,
      instructions: templateInstructions.value
    });
    activeCreationOperationId.value = output.operationId;
    creationRunning.value = true;
    logCreation(`Planned ${output.plannedTotal} item(s).`);
  }

  async function pickImportMarkdownFileAction() {
    const picked = await pickMarkdownFile();
    if (!picked.path) return;
    creationMode.value = 'import-md';
    const loaded = await loadMarkdownFile({ path: picked.path });
    importPath.value = loaded.path;
    importContent.value = loaded.content;
    if (!importName.value.trim()) {
      importName.value = loaded.path.split('/').pop()?.replace(/\.md$/i, '') ?? '';
    }
  }

  async function validateImportAction() {
    importValidation.value = await validateImportMarkdown({
      name: importName.value,
      slug: importSlug.value,
      content: importContent.value
    });
    return importValidation.value;
  }

  async function importMarkdownSkillAction() {
    const result = await validateImportAction();
    if (!result.canSubmit || creationRunning.value || running.value || pdfRunning.value) return;
    const output = await importSkillMarkdown({
      name: importName.value,
      slug: importSlug.value,
      content: importContent.value
    });
    activeCreationOperationId.value = output.operationId;
    creationRunning.value = true;
    logCreation(`Planned ${output.plannedTotal} item(s).`);
  }

  const onKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && pendingConflict.value) {
      void submitConflictResolution('skip');
    }
  };

  let unlisten: (() => void) | null = null;
  let unlistenPdf: (() => void) | null = null;
  onMounted(async () => {
    if (!isDesktopRuntime()) {
      error.value = 'Desktop runtime não detectado. Abra pelo Tauri com `npm run tauri:dev`.';
      return;
    }
    if (typeof window !== 'undefined') {
      const persistedFilter = window.localStorage?.getItem?.(storageFilterKey) ?? null;
      const persistedHidden = window.localStorage?.getItem?.(storageHiddenKey) ?? null;
      if (persistedFilter !== null) filter.value = persistedFilter;
      if (persistedHidden !== null) includeHidden.value = persistedHidden === 'true';
      window.addEventListener('keydown', onKeydown);
    }

    unlisten = await subscribeOperationEvents({
      onStarted(payload) {
        if (payload.mode === 'skills_move') {
          activeOperationId.value = payload.operationId;
          running.value = true;
          log(`Running [${payload.mode}]... (${payload.total} planned)`);
          return;
        }
        if (payload.mode === 'template' || payload.mode === 'import-md') {
          activeCreationOperationId.value = payload.operationId;
          creationRunning.value = true;
          logCreation(`Running [${payload.mode}]... (${payload.total} planned)`);
        }
      },
      onItemLog(payload) {
        if (payload.mode === 'skills_move' && running.value) {
          log(payload.message);
        }
        if ((payload.mode === 'template' || payload.mode === 'import-md') && creationRunning.value) {
          logCreation(payload.message);
        }
      },
      onItemResult(payload) {
        if (payload.mode === 'skills_move' && running.value) {
          if (payload.status === 'moved') log(`MOVED: ${payload.src} -> ${payload.dst}`);
          if (payload.status === 'error') log(`ERROR: ${payload.src} -> ${payload.dst}: ${payload.error ?? 'unknown'}`);
          if (payload.status === 'skipped') log(`SKIPPED: ${payload.slug}`);
          return;
        }

        if ((payload.mode === 'template' || payload.mode === 'import-md') && creationRunning.value) {
          if (payload.status === 'success') logCreation(`SUCCESS: ${payload.item} -> ${payload.dst}`);
          if (payload.status === 'failed') logCreation(`FAILED: ${payload.item}: ${payload.error ?? 'unknown'}`);
          if (payload.status === 'skipped') logCreation(`SKIPPED: ${payload.slug}`);
        }
      },
      onConflictRequired(payload) {
        pendingConflict.value = payload;
        conflictError.value = null;
        overwriteConfirmation.value = '';
        logCreation(`Conflict detected for '${payload.slug}'.`);
      },
      onFinished(payload) {
        if (payload.mode === 'skills_move') {
          running.value = false;
          activeOperationId.value = null;
          if (payload.cancelled) {
            log(`Cancelled after ${payload.attempted}/${payload.total}.`);
          } else {
            log(`Done: OK=${payload.ok} ERROR=${payload.error} SKIPPED=${payload.skipped} TOTAL=${payload.total}.`);
          }
          pendingConflict.value = null;
          refresh();
          return;
        }

        if (payload.mode === 'template' || payload.mode === 'import-md') {
          creationRunning.value = false;
          activeCreationOperationId.value = null;
          pendingConflict.value = null;
          if (payload.cancelled) {
            logCreation(`Cancelled after ${payload.attempted}/${payload.total}.`);
          } else {
            logCreation(`Done: OK=${payload.ok} ERROR=${payload.error} SKIPPED=${payload.skipped} TOTAL=${payload.total}.`);
          }
          refresh();
        }
      }
    });

    unlistenPdf = await subscribePdfEvents({
      onStarted(payload) {
        if (pdfOperationId.value && payload.operationId !== pdfOperationId.value) return;
        pdfProgress.value = `Running 0/${payload.total}`;
        logPdf(pdfProgress.value);
      },
      onFileProgress(payload) {
        if (pdfOperationId.value && payload.operationId !== pdfOperationId.value) return;
        pdfProgress.value = `Running ${payload.currentIndex}/${payload.total}: ${payload.file}`;
      },
      onFileLog(payload) {
        if (pdfOperationId.value && payload.operationId !== pdfOperationId.value) return;
        logPdf(`[${payload.file}] ${payload.message}`);
      },
      onFileResult(payload) {
        if (pdfOperationId.value && payload.operationId !== pdfOperationId.value) return;
        if (payload.status === 'success') logPdf(`SUCCESS: ${payload.file} -> ${payload.outputSkillPath ?? '-'}`);
        if (payload.status === 'failed') logPdf(`FAILED: ${payload.file}: ${payload.error ?? 'unknown error'}`);
      },
      onFinished(payload) {
        if (pdfOperationId.value && payload.operationId !== pdfOperationId.value) return;
        pdfRunning.value = false;
        pdfOperationId.value = null;
        pdfProgress.value = `Finished: OK=${payload.successCount} FAIL=${payload.failureCount} CANCELLED=${payload.cancelled}`;
        logPdf(pdfProgress.value);
        logCreation(pdfProgress.value);
        refresh();
      }
    });
  });

  onBeforeUnmount(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', onKeydown);
    }
    if (unlisten) unlisten();
    if (unlistenPdf) unlistenPdf();
  });

  function toggleSelected(direction: 'enable' | 'disable', slug: string, checked: boolean) {
    const target = direction === 'disable' ? selectedEnabled.value : selectedDisabled.value;
    if (checked) target.add(slug);
    else target.delete(slug);
    if (direction === 'disable') selectedEnabled.value = new Set(target);
    else selectedDisabled.value = new Set(target);
  }

  function toggleSelectedFromEvent(direction: 'enable' | 'disable', slug: string, event: Event) {
    const target = event.target as HTMLInputElement | null;
    toggleSelected(direction, slug, Boolean(target?.checked));
  }

  function isSelected(direction: 'enable' | 'disable', slug: string): boolean {
    const target = direction === 'disable' ? selectedEnabled.value : selectedDisabled.value;
    return target.has(slug);
  }

  function selectAllVisible(direction: 'enable' | 'disable') {
    if (direction === 'disable') {
      selectedEnabled.value = new Set(enabled.value.map((entry) => entry.slug));
    } else {
      selectedDisabled.value = new Set(disabled.value.map((entry) => entry.slug));
    }
  }

  function clearSelection(direction?: 'enable' | 'disable') {
    if (!direction || direction === 'disable') selectedEnabled.value = new Set();
    if (!direction || direction === 'enable') selectedDisabled.value = new Set();
  }

  function togglePdfSelected(file: string, checked: boolean) {
    if (checked) selectedPdfFiles.value.add(file);
    else selectedPdfFiles.value.delete(file);
    selectedPdfFiles.value = new Set(selectedPdfFiles.value);
  }

  function togglePdfSelectedFromEvent(file: string, event: Event) {
    const target = event.target as HTMLInputElement | null;
    togglePdfSelected(file, Boolean(target?.checked));
  }

  function setTemplateSlug(value: string) {
    templateSlug.value = value;
    templateSlugManuallyEdited.value = true;
  }

  function setImportSlug(value: string) {
    importSlug.value = value;
    importSlugManuallyEdited.value = true;
  }

  function switchToTemplateMode() {
    creationMode.value = 'template';
  }

  function clearCreationStatus() {
    creationStatusVisible.value = false;
  }

  const canConfirmOverwrite = computed(() => {
    if (!pendingConflict.value) return false;
    return overwriteConfirmation.value.trim() === pendingConflict.value.slug;
  });

  return {
    roots,
    enabled,
    disabled,
    filter,
    includeHidden,
    loading,
    error,
    logs,
    running,
    pendingConflict,
    overwriteConfirmation,
    conflictError,
    applyToAll,
    canConfirmOverwrite,
    selectedEnabled,
    selectedDisabled,
    pdfDirectory,
    pdfFiles,
    selectedPdfFiles,
    pdfMode,
    pdfOverwrite,
    pdfRunning,
    pdfProgress,
    pdfLogs,
    enabledCount,
    disabledCount,
    selectedEnabledCount,
    selectedDisabledCount,
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
    importPath,
    importContent,
    importName,
    importSlug,
    importValidation,
    refresh,
    runOperation,
    cancelActiveOperation,
    submitConflictResolution,
    toggleSelected,
    toggleSelectedFromEvent,
    isSelected,
    openSkillsFolderAction,
    selectAllVisible,
    clearSelection,
    selectPdfFiles,
    startPdfConversionAction,
    cancelPdfConversionAction,
    togglePdfSelected,
    togglePdfSelectedFromEvent,
    validateTemplateAction,
    createTemplateSkillAction,
    setTemplateSlug,
    switchToTemplateMode,
    pickImportMarkdownFileAction,
    validateImportAction,
    importMarkdownSkillAction,
    setImportSlug,
    clearCreationStatus
  };
}

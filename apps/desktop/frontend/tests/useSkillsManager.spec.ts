import { defineComponent, nextTick } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useSkillsManager } from '~/composables/useSkillsManager';

const tauriMocks = vi.hoisted(() => {
  return {
    getRoots: vi.fn(),
    listSkills: vi.fn(),
    validateTemplate: vi.fn(),
    createSkillFromTemplate: vi.fn(),
    validateImportMarkdown: vi.fn(),
    importSkillMarkdown: vi.fn(),
    subscribeOperationEvents: vi.fn(),
    subscribePdfEvents: vi.fn()
  };
});

vi.mock('~/services/tauri', () => ({
  isDesktopRuntime: () => true,
  cancelOperation: vi.fn(async () => ({ accepted: true })),
  cancelPdfConversion: vi.fn(async () => ({ accepted: true })),
  createSkillFromTemplate: tauriMocks.createSkillFromTemplate,
  getRoots: tauriMocks.getRoots,
  importSkillMarkdown: tauriMocks.importSkillMarkdown,
  listSkills: tauriMocks.listSkills,
  loadMarkdownFile: vi.fn(async () => ({ path: '/tmp/a.md', content: '# hi' })),
  openSkillsFolder: vi.fn(async () => ({ opened: true, path: '/tmp' })),
  pickMarkdownFile: vi.fn(async () => ({ path: '/tmp/a.md' })),
  pickPdfFiles: vi.fn(async () => ({ paths: [] })),
  resolveConflict: vi.fn(async () => ({ accepted: true })),
  startPdfConversion: vi.fn(async () => ({ operationId: 'pdf-op', plannedTotal: 0 })),
  startOperation: vi.fn(async () => ({ operationId: 'move-op', plannedTotal: 0 })),
  subscribeOperationEvents: tauriMocks.subscribeOperationEvents,
  subscribePdfEvents: tauriMocks.subscribePdfEvents,
  validateImportMarkdown: tauriMocks.validateImportMarkdown,
  validateTemplate: tauriMocks.validateTemplate
}));

function mountHarness() {
  let api: ReturnType<typeof useSkillsManager> | null = null;
  const Harness = defineComponent({
    setup() {
      api = useSkillsManager();
      return () => null;
    }
  });
  mount(Harness);
  return () => {
    if (!api) throw new Error('composable not mounted');
    return api;
  };
}

describe('useSkillsManager creation flow', () => {
  beforeEach(() => {
    tauriMocks.getRoots.mockResolvedValue({
      base: '/Users/test/.codex',
      enabledRoot: '/Users/test/.codex/skills',
      disabledRoot: '/Users/test/.codex/skills_disabled'
    });
    tauriMocks.listSkills.mockResolvedValue({ enabled: [], disabled: [] });
    tauriMocks.validateTemplate.mockResolvedValue({ issues: [], canSubmit: true });
    tauriMocks.createSkillFromTemplate.mockResolvedValue({ operationId: 'tpl-op', plannedTotal: 1 });
    tauriMocks.validateImportMarkdown.mockResolvedValue({ issues: [], canSubmit: true });
    tauriMocks.importSkillMarkdown.mockResolvedValue({ operationId: 'imp-op', plannedTotal: 1 });
    tauriMocks.subscribeOperationEvents.mockResolvedValue(() => undefined);
    tauriMocks.subscribePdfEvents.mockResolvedValue(() => undefined);
  });

  it('auto-generates slug and preserves manual slug lock', async () => {
    const getApi = mountHarness();
    await flushPromises();
    const api = getApi();
    api.templateName.value = 'Minha Skill Nova';
    await nextTick();
    expect(api.templateSlug.value).toBe('minha-skill-nova');

    api.setTemplateSlug('custom-slug');
    api.templateName.value = 'Nome Alterado';
    await nextTick();
    expect(api.templateSlug.value).toBe('custom-slug');
  });

  it('blocks template create when validation has error', async () => {
    tauriMocks.validateTemplate.mockResolvedValueOnce({
      canSubmit: false,
      issues: [{ level: 'error', code: 'name_required', message: 'Skill name is required.', field: 'name' }]
    });
    const getApi = mountHarness();
    await flushPromises();
    const api = getApi();
    api.templateName.value = '';
    api.templateInstructions.value = '';
    await api.createTemplateSkillAction();

    expect(tauriMocks.createSkillFromTemplate).not.toHaveBeenCalled();
  });

  it('creates template skill when validation passes', async () => {
    const getApi = mountHarness();
    await flushPromises();
    const api = getApi();
    api.templateName.value = 'Skill A';
    api.templateDescription.value = 'Desc';
    api.templateInstructions.value = 'Do this';
    api.templateSlug.value = 'skill-a';
    await api.createTemplateSkillAction();

    expect(tauriMocks.createSkillFromTemplate).toHaveBeenCalledWith({
      name: 'Skill A',
      slug: 'skill-a',
      description: 'Desc',
      instructions: 'Do this'
    });
    expect(api.creationRunning.value).toBe(true);
  });

  it('allows import flow with warnings (no errors)', async () => {
    tauriMocks.validateImportMarkdown.mockResolvedValueOnce({
      canSubmit: true,
      issues: [{ level: 'warning', code: 'frontmatter_missing', message: 'YAML frontmatter is recommended.' }]
    });
    const getApi = mountHarness();
    await flushPromises();
    const api = getApi();
    api.importName.value = 'Imported';
    api.importSlug.value = 'imported';
    api.importContent.value = '# title\ncontent';
    await api.importMarkdownSkillAction();

    expect(tauriMocks.importSkillMarkdown).toHaveBeenCalled();
    expect(api.creationRunning.value).toBe(true);
  });
});

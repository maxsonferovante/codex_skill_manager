import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type {
  CancelOperationInput,
  ListSkillsInput,
  ListSkillsOutput,
  OperationConflictRequiredEvent,
  OperationFinishedEvent,
  OperationItemLogEvent,
  OperationItemResultEvent,
  OpenSkillsFolderOutput,
  OperationStartedEvent,
  PickPdfFilesOutput,
  PickMarkdownFileOutput,
  LoadMarkdownFileInput,
  LoadMarkdownFileOutput,
  ValidateTemplateInput,
  ValidateTemplateOutput,
  CreateSkillFromTemplateInput,
  ValidateImportMarkdownInput,
  ValidateImportMarkdownOutput,
  ImportSkillMarkdownInput,
  PdfFileLogEvent,
  PdfFileProgressEvent,
  PdfFileResultEvent,
  PdfFinishedEvent,
  PdfStartedEvent,
  ResolveConflictInput,
  RootsInfo,
  ScanPdfsInput,
  ScanPdfsOutput,
  StartPdfConversionInput,
  StartPdfConversionOutput,
  StartOperationInput,
  StartOperationOutput
} from '~/types/contracts';

export function isDesktopRuntime(): boolean {
  return isTauri();
}

export async function invokeCommand<T>(cmd: string, payload?: unknown): Promise<T> {
  if (!isTauri()) {
    throw new Error(`Command '${cmd}' requires Tauri desktop runtime. Use 'npm run tauri:dev'.`);
  }
  return invoke<T>(cmd, payload as Record<string, unknown> | undefined);
}

export async function getRoots(): Promise<RootsInfo> {
  return invokeCommand<RootsInfo>('get_roots');
}

export async function listSkills(input: ListSkillsInput): Promise<ListSkillsOutput> {
  return invokeCommand<ListSkillsOutput>('list_skills', { input });
}

export async function startOperation(input: StartOperationInput): Promise<StartOperationOutput> {
  return invokeCommand<StartOperationOutput>('start_operation', { input });
}

export async function cancelOperation(input: CancelOperationInput): Promise<{ accepted: boolean }> {
  return invokeCommand<{ accepted: boolean }>('cancel_operation', { input });
}

export async function resolveConflict(input: ResolveConflictInput): Promise<{ accepted: boolean }> {
  return invokeCommand<{ accepted: boolean }>('resolve_conflict', { input });
}

export async function openSkillsFolder(): Promise<OpenSkillsFolderOutput> {
  return invokeCommand<OpenSkillsFolderOutput>('open_skills_folder');
}

export async function scanPdfsInDirectory(input: ScanPdfsInput): Promise<ScanPdfsOutput> {
  return invokeCommand<ScanPdfsOutput>('scan_pdfs_in_directory', { input });
}

export async function startPdfConversion(input: StartPdfConversionInput): Promise<StartPdfConversionOutput> {
  return invokeCommand<StartPdfConversionOutput>('start_pdf_conversion', { input });
}

export async function cancelPdfConversion(input: { operationId: string }): Promise<{ accepted: boolean }> {
  return invokeCommand<{ accepted: boolean }>('cancel_pdf_conversion', { input });
}

export async function pickPdfFiles(): Promise<PickPdfFilesOutput> {
  return invokeCommand<PickPdfFilesOutput>('pick_pdf_files');
}

export async function pickMarkdownFile(): Promise<PickMarkdownFileOutput> {
  return invokeCommand<PickMarkdownFileOutput>('pick_markdown_file');
}

export async function loadMarkdownFile(input: LoadMarkdownFileInput): Promise<LoadMarkdownFileOutput> {
  return invokeCommand<LoadMarkdownFileOutput>('load_markdown_file', { input });
}

export async function validateTemplate(input: ValidateTemplateInput): Promise<ValidateTemplateOutput> {
  return invokeCommand<ValidateTemplateOutput>('validate_template', { input });
}

export async function createSkillFromTemplate(input: CreateSkillFromTemplateInput): Promise<StartOperationOutput> {
  return invokeCommand<StartOperationOutput>('create_skill_from_template', { input });
}

export async function validateImportMarkdown(
  input: ValidateImportMarkdownInput
): Promise<ValidateImportMarkdownOutput> {
  return invokeCommand<ValidateImportMarkdownOutput>('validate_import_markdown', { input });
}

export async function importSkillMarkdown(input: ImportSkillMarkdownInput): Promise<StartOperationOutput> {
  return invokeCommand<StartOperationOutput>('import_skill_markdown', { input });
}

export async function subscribeOperationEvents(handlers: {
  onStarted: (payload: OperationStartedEvent) => void;
  onItemLog: (payload: OperationItemLogEvent) => void;
  onItemResult: (payload: OperationItemResultEvent) => void;
  onConflictRequired: (payload: OperationConflictRequiredEvent) => void;
  onFinished: (payload: OperationFinishedEvent) => void;
}): Promise<() => void> {
  if (!isTauri()) {
    return () => undefined;
  }
  const unlistenStarted = await listen<OperationStartedEvent>('operation:started', (event) =>
    handlers.onStarted(event.payload)
  );
  const unlistenItemLog = await listen<OperationItemLogEvent>('operation:item_log', (event) =>
    handlers.onItemLog(event.payload)
  );
  const unlistenItemResult = await listen<OperationItemResultEvent>('operation:item_result', (event) =>
    handlers.onItemResult(event.payload)
  );
  const unlistenConflict = await listen<OperationConflictRequiredEvent>('operation:conflict_required', (event) =>
    handlers.onConflictRequired(event.payload)
  );
  const unlistenFinished = await listen<OperationFinishedEvent>('operation:finished', (event) =>
    handlers.onFinished(event.payload)
  );
  return () => {
    unlistenStarted();
    unlistenItemLog();
    unlistenItemResult();
    unlistenConflict();
    unlistenFinished();
  };
}

export async function subscribePdfEvents(handlers: {
  onStarted: (payload: PdfStartedEvent) => void;
  onFileProgress: (payload: PdfFileProgressEvent) => void;
  onFileLog: (payload: PdfFileLogEvent) => void;
  onFileResult: (payload: PdfFileResultEvent) => void;
  onFinished: (payload: PdfFinishedEvent) => void;
}): Promise<() => void> {
  if (!isTauri()) {
    return () => undefined;
  }
  const unlistenStarted = await listen<PdfStartedEvent>('pdf:started', (event) => handlers.onStarted(event.payload));
  const unlistenProgress = await listen<PdfFileProgressEvent>('pdf:file_progress', (event) =>
    handlers.onFileProgress(event.payload)
  );
  const unlistenLog = await listen<PdfFileLogEvent>('pdf:file_log', (event) => handlers.onFileLog(event.payload));
  const unlistenResult = await listen<PdfFileResultEvent>('pdf:file_result', (event) => handlers.onFileResult(event.payload));
  const unlistenFinished = await listen<PdfFinishedEvent>('pdf:finished', (event) => handlers.onFinished(event.payload));
  return () => {
    unlistenStarted();
    unlistenProgress();
    unlistenLog();
    unlistenResult();
    unlistenFinished();
  };
}

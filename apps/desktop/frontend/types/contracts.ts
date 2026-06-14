export type Direction = 'enable' | 'disable';
export type OperationMode = 'selected' | 'allVisible';
export type ConflictAction = 'rename' | 'overwrite' | 'skip';
export type ItemStatus = 'moved' | 'error' | 'skipped';

export interface SkillEntry {
  slug: string;
  path: string;
}

export interface RootsInfo {
  base: string;
  enabledRoot: string;
  disabledRoot: string;
}

export interface ManagerError {
  code: string;
  message: string;
  context?: {
    operationId?: string;
    src?: string;
    dst?: string;
  };
}

export interface ListSkillsInput {
  includeHidden: boolean;
  filter?: string;
}

export interface ListSkillsOutput {
  enabled: SkillEntry[];
  disabled: SkillEntry[];
}

export interface StartOperationInput {
  direction: Direction;
  mode: OperationMode;
  slugs?: string[];
  includeHidden: boolean;
  filter?: string;
}

export interface StartOperationOutput {
  operationId: string;
  plannedTotal: number;
}

export interface CancelOperationInput {
  operationId: string;
}

export interface ResolveConflictInput {
  operationId: string;
  slug: string;
  action: ConflictAction;
  applyToAll: boolean;
  overwriteConfirmationSlug?: string;
}

export interface OpenSkillsFolderOutput {
  opened: boolean;
  path: string;
  message?: string;
}

export interface OperationStartedEvent {
  operationId: string;
  mode: string;
  total: number;
  startedAt: string;
}

export interface OperationItemResultEvent {
  operationId: string;
  mode: string;
  item: string;
  src: string;
  dst: string;
  status: ItemStatus | 'success' | 'failed';
  slug: string;
  error?: string;
  at: string;
}

export interface OperationItemLogEvent {
  operationId: string;
  mode: string;
  item: string;
  level: string;
  message: string;
  at: string;
}

export interface OperationConflictRequiredEvent {
  operationId: string;
  slug: string;
  src: string;
  dst: string;
  allowApplyToAll: true;
}

export interface OperationCancelRequestedEvent {
  operationId: string;
  at: string;
}

export interface OperationFinishedEvent {
  operationId: string;
  mode: string;
  total: number;
  attempted: number;
  ok: number;
  error: number;
  skipped: number;
  cancelled: boolean;
  finishedAt: string;
}

export type PdfExtractionMode = 'technical' | 'text';

export interface ScanPdfsInput {
  directory: string;
}

export interface ScanPdfsOutput {
  files: string[];
}

export interface PickPdfFilesOutput {
  paths: string[];
}

export interface PickMarkdownFileOutput {
  path?: string;
}

export interface LoadMarkdownFileInput {
  path: string;
}

export interface LoadMarkdownFileOutput {
  path: string;
  content: string;
}

export type ValidationLevel = 'error' | 'warning' | 'info';

export interface ValidationIssue {
  level: ValidationLevel;
  code: string;
  message: string;
  field?: string;
}

export interface ValidateTemplateInput {
  name: string;
  slug: string;
  description: string;
  instructions: string;
}

export interface ValidateTemplateOutput {
  issues: ValidationIssue[];
  canSubmit: boolean;
}

export interface CreateSkillFromTemplateInput {
  name: string;
  slug: string;
  description: string;
  instructions: string;
}

export interface ValidateImportMarkdownInput {
  name: string;
  slug: string;
  content: string;
}

export interface ValidateImportMarkdownOutput {
  issues: ValidationIssue[];
  canSubmit: boolean;
}

export interface ImportSkillMarkdownInput {
  name: string;
  slug: string;
  content: string;
}

export interface StartPdfConversionInput {
  directory: string;
  selectedFiles?: string[];
  mode: PdfExtractionMode;
  targetChunkTokens?: number;
  maxChunkTokens?: number;
  overwrite: boolean;
}

export interface StartPdfConversionOutput {
  operationId: string;
  plannedTotal: number;
}

export interface CancelPdfConversionInput {
  operationId: string;
}

export interface PdfStartedEvent {
  operationId: string;
  total: number;
}

export interface PdfFileProgressEvent {
  operationId: string;
  currentIndex: number;
  total: number;
  file: string;
}

export interface PdfFileLogEvent {
  operationId: string;
  file: string;
  message: string;
}

export interface PdfFileResultEvent {
  operationId: string;
  file: string;
  status: 'success' | 'failed';
  outputSkillPath?: string;
  error?: string;
}

export interface PdfFinishedEvent {
  operationId: string;
  successCount: number;
  failureCount: number;
  cancelled: boolean;
}

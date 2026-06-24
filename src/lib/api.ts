import { invoke } from '@tauri-apps/api/core';
import { token, clearAuth } from './stores/auth';
import { get } from 'svelte/store';

function getToken(): string {
  const t = get(token);
  if (!t) throw new Error('Not authenticated');
  return t;
}

async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(command, { token: getToken(), ...args });
  } catch (e: unknown) {
    const msg = typeof e === 'string' ? e : (e instanceof Error ? e.message : 'Unknown error');
    if (msg.includes('Session expired or invalid') || msg.includes('Session expired')) {
      clearAuth();
    }
    throw new Error(msg);
  }
}

// Auth (login doesn't need token)
export async function login(username: string, password: string) {
  try {
    return await invoke<{ token: string; user: any; must_change_password: boolean }>('login', { username, password });
  } catch (e: unknown) {
    const msg = typeof e === 'string' ? e : (e instanceof Error ? e.message : 'Login failed');
    throw new Error(msg);
  }
}

export async function changePassword(newPassword: string) {
  return call<void>('change_password', { newPassword });
}

export async function getCurrentUser() {
  return call<any>('get_current_user');
}

export async function logout() {
  return call<void>('logout');
}

export async function listUsers() {
  return call<any[]>('list_users');
}

export async function createUser(request: any) {
  return call<any>('create_user', { request });
}

export async function updateUserRole(userId: string, newRole: string) {
  return call<void>('update_user_role', { userId, newRole });
}

// Specimens
export async function listSpecimens(page = 1, perPage = 50) {
  return call<any>('list_specimens', { page, perPage });
}

export async function getSpecimen(id: string) {
  return call<any>('get_specimen', { id });
}

export async function createSpecimen(request: any) {
  return call<any>('create_specimen', { request });
}

export async function updateSpecimen(request: any) {
  return call<any>('update_specimen', { request });
}

export async function deleteSpecimen(id: string) {
  return call<void>('delete_specimen', { id });
}

export async function searchSpecimens(paramsInput: any) {
  return call<any>('search_specimens', { paramsInput });
}

export async function getSpecimenStats() {
  return call<any>('get_specimen_stats');
}

export async function getSpecimenFamily(id: string) {
  return call<any[]>('get_specimen_family', { id });
}

export async function bulkArchiveSpecimens(ids: string[]) {
  return call<number>('bulk_archive_specimens', { ids });
}

export async function bulkUpdateLocation(ids: string[], location: string) {
  return call<number>('bulk_update_location', { ids, location });
}

export async function bulkUpdateStage(ids: string[], stage: string) {
  return call<number>('bulk_update_stage', { ids, stage });
}

export async function splitSpecimen(request: {
  parent_specimen_id: string;
  date: string;
  children: Array<{
    accession_number?: string;
    location?: string;
    media_batch_id?: string;
    vessel_type?: string;
    notes?: string;
    health_status?: string;
    stage?: string;
    reminder_days?: number | null;
  }>;
  observations?: string;
  notes?: string;
  employee_id?: string;
  health_status?: string;
  contamination_flag?: boolean;
  contamination_notes?: string;
  temperature_c?: number;
  ph?: number;
  light_cycle?: string;
}) {
  return call<{ archived_parent_id: string; children: Array<{ id: string; accession_number: string }> }>(
    'split_specimen', { request }
  );
}

export async function previewSplitAccessions(parentId: string, count: number) {
  return call<string[]>('preview_split_accessions', { parentId, count });
}

export async function createDraftMediaBatch(name: string) {
  return call<any>('create_draft_media_batch', { name });
}

// Media
export async function listMedia() {
  return call<any[]>('list_media');
}

export async function getMediaBatch(id: string) {
  return call<any>('get_media_batch', { id });
}

export async function createMediaBatch(request: any) {
  return call<any>('create_media_batch', { request });
}

export async function updateMediaBatch(request: any) {
  return call<any>('update_media_batch', { request });
}

export async function deleteMediaBatch(id: string) {
  return call<void>('delete_media_batch', { id });
}

// Subcultures
export async function listSubcultures(specimenId: string, page = 1, perPage = 200) {
  const resp = await call<{ items: any[]; total: number; page: number; per_page: number; total_pages: number }>(
    'list_subcultures', { specimenId, page, perPage }
  );
  return resp.items;
}

export async function listAllSubcultures() {
  return call<any[]>('list_all_subcultures');
}

export async function createSubculture(request: any) {
  return call<any>('create_subculture', { request });
}

export async function recordSpecimenDeath(request: {
  specimen_id: string;
  date: string;
  observations?: string;
  notes?: string;
  employee_id?: string;
}) {
  return call<any>('record_specimen_death', { request });
}

export async function updateSubculture(request: any) {
  return call<void>('update_subculture', { request });
}

export async function getContaminationStats() {
  return call<any>('get_contamination_stats');
}

export async function getSubcultureSchedule() {
  return call<any[]>('get_subculture_schedule');
}

// Reminders
export async function listReminders() {
  return call<any[]>('list_reminders');
}

export async function getActiveReminders() {
  return call<any[]>('get_active_reminders');
}

export async function createReminder(request: any) {
  return call<any>('create_reminder', { request });
}

export async function updateReminder(request: any) {
  return call<void>('update_reminder', { request });
}

export async function dismissReminder(id: string, snooze: boolean, snoozeDays?: number) {
  return call<void>('dismiss_reminder', { id, snooze, snoozeDays: snoozeDays ?? null });
}

// Compliance
export async function listComplianceRecords(specimenId?: string, page = 1, perPage = 100) {
  return call<any>('list_compliance_records', { specimenId: specimenId ?? null, page, perPage });
}

export async function createComplianceRecord(request: any) {
  return call<any>('create_compliance_record', { request });
}

export async function updateComplianceRecord(request: any) {
  return call<void>('update_compliance_record', { request });
}

export async function getComplianceFlags() {
  return call<any[]>('get_compliance_flags');
}

// Species
export async function listSpecies() {
  return call<any[]>('list_species');
}

export async function listProjects() {
  return call<any[]>('list_projects');
}

export async function createSpecies(request: any) {
  return call<any>('create_species', { request });
}

export async function updateSpecies(request: any) {
  return call<void>('update_species', { request });
}

// Audit
export async function getAuditLog(search: any = {}) {
  return call<any>('get_audit_log', { search });
}

export async function verifyAuditEntry(entryId: string) {
  return call<any>('verify_audit_entry', { entryId });
}

export async function verifyAuditLineage(lineageId: string) {
  return call<any>('verify_audit_lineage', { lineageId });
}

export async function createAuditCheckpoint(lineageId: string, startSeq?: number, endSeq?: number) {
  return call<any>('create_audit_checkpoint', { lineageId, startSeq, endSeq });
}

export async function verifyAgainstCheckpoint(checkpointId: string) {
  return call<any>('verify_against_checkpoint', { checkpointId });
}

export async function listAuditCheckpoints(lineageId?: string) {
  return call<any[]>('list_audit_checkpoints', { lineageId });
}

// WP-21: Portable Merkle proof export and standalone verification
export async function exportAuditProof(checkpointId: string) {
  return call<string>('export_audit_proof', { checkpointId });
}

export async function verifyExportedProof(proofJson: string) {
  return call<any>('verify_exported_proof', { proofJson });
}

// WP-21: Auto-checkpoint configuration
export async function getAutoCheckpointConfig() {
  return call<any>('get_auto_checkpoint_config');
}

export async function setAutoCheckpointConfig(config: { enabled: boolean; interval: number; on_backup: boolean }) {
  return call<void>('set_auto_checkpoint_config', { config });
}

export async function runAutoCheckpoint() {
  return call<any>('run_auto_checkpoint');
}

// Import
export async function importXlsx(payload: {
  specimens: string[][];
  subcultures: string[][];
  media: string[][];
  prepared_solutions: string[][];
  inventory: string[][];
  compliance: string[][];
}, dryRun: boolean) {
  return call<any>('import_xlsx', { payload, dryRun });
}

// Export
export async function exportSpecimensCsv() {
  return call<string>('export_specimens_csv');
}

export async function exportSpecimensJson() {
  return call<string>('export_specimens_json');
}

// Inventory
export async function listInventory() {
  return call<any[]>('list_inventory');
}

export async function createInventoryItem(request: any) {
  return call<any>('create_inventory_item', { request });
}

export async function updateInventoryItem(request: any) {
  return call<any>('update_inventory_item', { request });
}

export async function deleteInventoryItem(id: string) {
  return call<void>('delete_inventory_item', { id });
}

export async function adjustStock(id: string, adjustment: number, reason?: string) {
  return call<any>('adjust_stock', { id, adjustment, reason: reason ?? null });
}

export async function getLowStockAlerts() {
  return call<any[]>('get_low_stock_alerts');
}

// Admin / Dev tools
export async function resetDatabase(confirmation: string) {
  return call<string>('reset_database', { confirmation });
}

export async function loadDemoData() {
  return call<string>('load_demo_data');
}

export async function getLabProfile() {
  return call<string>('get_lab_profile');
}

export async function setLabProfile(profile: string, confirmation?: string) {
  return call<void>('set_lab_profile', { profile, confirmation: confirmation ?? null });
}

// Vocabulary lookups (WP-23 / WP-24) — returns entries for the active lab profile.
export type VocabEntry = { id: number; code: string; label: string; sort_order: number };
export type StageEntry = VocabEntry & { is_terminal: boolean };

export async function listStages() {
  return call<StageEntry[]>('list_stages');
}

export async function listPropagationMethods() {
  return call<VocabEntry[]>('list_propagation_methods');
}

export async function listHormoneTypes() {
  return call<VocabEntry[]>('list_hormone_types');
}

export async function listComplianceRecordTypes() {
  return call<VocabEntry[]>('list_compliance_record_types');
}

export async function listComplianceAgencies() {
  return call<VocabEntry[]>('list_compliance_agencies');
}

export async function listInventoryCategories() {
  return call<VocabEntry[]>('list_inventory_categories');
}

// Prepared Solutions
export async function listPreparedSolutions() {
  return call<any[]>('list_prepared_solutions');
}

export async function createPreparedSolution(request: any) {
  return call<any>('create_prepared_solution', { request });
}

export async function updatePreparedSolution(request: any) {
  return call<void>('update_prepared_solution', { request });
}

export async function deletePreparedSolution(id: string) {
  return call<void>('delete_prepared_solution', { id });
}

// Backup
export async function createBackup(destination?: string) {
  return call<string>('create_backup', { destination: destination ?? null });
}

export async function listBackups() {
  return call<any[]>('list_backups');
}

export async function restoreBackup(backupPath: string) {
  return call<string>('restore_backup', { backupPath });
}

// Error Logs
export async function logError(request: {
  title: string;
  message: string;
  module?: string;
  severity?: string;
  form_payload?: string;
  stack_trace?: string;
}) {
  return call<any>('log_error', { request });
}

export async function listErrorLogs(search: {
  severity?: string;
  module?: string;
  unread_only?: boolean;
  page?: number;
  per_page?: number;
} = {}) {
  return call<any>('list_error_logs', { search });
}

export async function getUnreadErrorCount() {
  return call<number>('get_unread_error_count');
}

export async function markErrorsRead() {
  return call<void>('mark_errors_read');
}

export async function clearErrorLogs() {
  return call<void>('clear_error_logs');
}

// Attachments
export async function listAttachments(entityType: string, entityId: string) {
  return call<any[]>('list_attachments', { entityType, entityId });
}

export async function uploadAttachment(
  entityType: string,
  entityId: string,
  fileName: string,
  mimeType: string,
  dataB64: string,
  description?: string,
) {
  return call<any>('upload_attachment', { entityType, entityId, fileName, mimeType, dataB64, description: description ?? null });
}

export async function getAttachmentData(id: string) {
  return call<string>('get_attachment_data', { id });
}

export async function deleteAttachment(id: string) {
  return call<void>('delete_attachment', { id });
}

// Work Queue
export async function getWorkQueue() {
  return call<any[]>('get_work_queue');
}

// Strains (WP-28)
export async function createStrain(request: {
  species_id: string;
  name: string;
  code: string;
  strain_type?: string;
}) {
  return call<any>('create_strain', { request });
}

export async function getStrain(id: string) {
  return call<any>('get_strain', { id });
}

export async function listStrainsBySpecies(speciesId: string) {
  return call<any[]>('list_strains_by_species', { speciesId });
}

export async function updateStrain(request: {
  id: string;
  name?: string;
  code?: string;
  strain_type?: string;
}) {
  return call<any>('update_strain', { request });
}

export async function archiveStrain(id: string) {
  return call<void>('archive_strain', { id });
}

export async function updateStrainStatus(request: {
  id: string;
  status: string;
  claimed_by?: string;
  claimed_at?: string;
  confirmation_basis?: string;
  genomic_fingerprint?: string;
}) {
  return call<any>('update_strain_status', { request });
}

export async function createHybridizationEvent(request: {
  parent_a_id: string;
  parent_b_id: string;
  name: string;
  code: string;
  notes?: string;
  generation_label?: string;
  admin_override_cross_species?: boolean;
  admin_override_reason?: string;
}) {
  return call<{ hybrid_strain_id: string; event_id: string }>('create_hybridization_event', { request });
}

export interface SuggestGenerationLabelResponse {
  suggested_label: string | null;
  is_backcross: boolean;
  backcross_depth: number | null;
  backcross_ancestor_id: string | null;
}

export async function suggestGenerationLabel(parentAId: string, parentBId: string) {
  return call<SuggestGenerationLabelResponse>('suggest_generation_label', { parentAId, parentBId });
}

export interface GenerationalStats {
  generation_label: string;
  specimen_count: number;
  healthy_count: number;
  problem_count: number;
}

export async function getGenerationalStats(strainId: string) {
  return call<GenerationalStats[]>('get_generational_stats', { strainId });
}

// Taxa (WP-35) — hierarchical taxonomy backbone (Genus → Kingdom).
// Taxa records are classification-only and carry no audit-chain data.

export type TaxonRank = 'kingdom' | 'phylum' | 'class' | 'order' | 'family' | 'genus';

export interface Taxon {
  id: string;
  rank: TaxonRank;
  name: string;
  parent_id: string | null;
  ncbi_taxon_id: number | null;
  ncbi_updated_at: string | null;
  local_override: boolean;
  taxon_path: string | null;
  created_at: string;
  updated_at: string;
}

export interface SpeciesNodeSummary {
  id: string;
  genus: string;
  species_name: string;
  common_name: string | null;
  species_code: string;
  strain_count: number;
  specimen_count: number;
}

export interface TaxonNode {
  taxon: Taxon;
  strain_count: number;
  specimen_count: number;
  species: SpeciesNodeSummary[];
  children: TaxonNode[];
}

export async function createTaxon(request: {
  rank: TaxonRank;
  name: string;
  parent_id?: string;
  ncbi_taxon_id?: number;
  local_override?: boolean;
}) {
  return call<Taxon>('create_taxon', { request });
}

export async function getTaxon(id: string) {
  return call<Taxon>('get_taxon', { id });
}

export async function updateTaxon(request: {
  id: string;
  name?: string;
  parent_id?: string;
  ncbi_taxon_id?: number;
  ncbi_updated_at?: string;
  local_override?: boolean;
}) {
  return call<void>('update_taxon', { request });
}

export async function listTaxaByRank(rank: TaxonRank) {
  return call<Taxon[]>('list_taxa_by_rank', { rank });
}

export async function getTaxonDescendants(id: string) {
  return call<TaxonNode>('get_taxon_descendants', { id });
}

// WP-39 — Advanced taxonomy navigator

export interface TaxonColumnItem {
  id: string;
  rank: string;
  name: string;
  parent_id: string | null;
  ncbi_taxon_id: number | null;
  local_override: boolean;
  strain_count: number;
  specimen_count: number;
}

export interface TaxonomySearchResult {
  result_type: 'taxon' | 'species' | 'strain' | 'specimen';
  id: string;
  display_name: string;
  secondary: string;
  taxon_ids: string[];
  species_id: string | null;
  strain_id: string | null;
}

export async function getTaxonColumn(parentId?: string) {
  return call<TaxonColumnItem[]>('get_taxon_column', { parentId: parentId ?? null });
}

export async function listSpeciesForTaxon(taxonId: string) {
  return call<SpeciesNodeSummary[]>('list_species_for_taxon', { taxonId });
}

export async function searchTaxonomy(query: string) {
  return call<TaxonomySearchResult[]>('search_taxonomy', { query });
}

// NCBI Taxonomy (WP-36) — import & ongoing sync.

export interface NcbiTaxonRecord {
  ncbi_taxon_id: number;
  name: string;
  rank: string;
  parent_ncbi_id: number | null;
}

export interface NcbiSyncLog {
  id: string;
  sync_type: 'import' | 'update' | 'conflict';
  taxon_id: string | null;
  ncbi_taxon_id: number | null;
  conflict_details: string | null;
  resolved_at: string | null;
  resolved_by: string | null;
  resolution: 'kept_local' | 'accepted_ncbi' | 'merged' | null;
  created_at: string;
}

export interface NcbiConflictSummary {
  sync_log_id: string | null;
  taxon_id: string | null;
  ncbi_taxon_id: number;
  local_name: string | null;
  ncbi_name: string;
  conflict_details: string;
}

export interface ImportNcbiTaxonomyResult {
  imported: number;
  updated: number;
  skipped_overrides: number;
  conflicts: NcbiConflictSummary[];
  dry_run: boolean;
}

export async function importNcbiTaxonomy(taxa: NcbiTaxonRecord[], dryRun: boolean) {
  return call<ImportNcbiTaxonomyResult>('import_ncbi_taxonomy', {
    request: { taxa, dry_run: dryRun },
  });
}

export async function resolveNcbiConflict(
  syncLogId: string,
  resolution: 'kept_local' | 'accepted_ncbi' | 'merged'
) {
  return call<void>('resolve_ncbi_conflict', {
    request: { sync_log_id: syncLogId, resolution },
  });
}

export async function syncNcbiTaxon(record: NcbiTaxonRecord) {
  return call<string>('sync_ncbi_taxon', { record });
}

export async function listNcbiSyncLog(pendingOnly: boolean, limit?: number) {
  return call<NcbiSyncLog[]>('list_ncbi_sync_log', {
    pendingOnly,
    limit: limit ?? null,
  });
}

// Pedigree (WP-37) — multi-generational strain ancestry and descendant trees.
// These functions walk the strain hybridization graph (strain_parents) only.
// They do NOT traverse specimen culture lineage (specimens.parent_specimen_id).

export interface StrainSummary {
  id: string;
  name: string;
  code: string;
  strain_type: string;
  status: string;
  is_hybrid: boolean;
  is_archived: boolean;
  specimen_count: number;
}

export interface PedigreeEdge {
  parent_strain_id: string;
  parent_role: string | null;
  parent_chain_seq_at_creation: number | null;
  event_id: string | null;
  event_notes: string | null;
}

export interface PedigreeNode {
  strain: StrainSummary;
  depth: number;
  edge: PedigreeEdge | null;
  parents: PedigreeNode[];
  children: PedigreeNode[];
}

export interface SpecimenSummary {
  id: string;
  accession_number: string;
  stage: string;
  location: string | null;
  is_archived: boolean;
  strain_id: string;
  created_at: string;
}

export interface StrainSpecimenTree {
  strain: StrainSummary;
  specimens: SpecimenSummary[];
  descendant_trees: StrainSpecimenTree[];
}

export interface HybridizationEventRecord {
  id: string;
  hybrid_strain_id: string;
  parent_a_strain_id: string;
  parent_b_strain_id: string;
  parent_a_chain_seq: number;
  parent_b_chain_seq: number;
  notes: string | null;
  created_at: string;
}

export interface PedigreeExport {
  root_strain_id: string;
  exported_at: string;
  strains: StrainSummary[];
  hybridization_events: HybridizationEventRecord[];
}

export async function getStrainAncestry(strainId: string, maxDepth?: number) {
  return call<PedigreeNode>('get_strain_ancestry', {
    strainId,
    maxDepth: maxDepth ?? null,
  });
}

export async function getStrainDescendants(strainId: string, maxDepth?: number) {
  return call<PedigreeNode>('get_strain_descendants', {
    strainId,
    maxDepth: maxDepth ?? null,
  });
}

export async function getStrainSpecimenTree(strainId: string, includeDescendants?: boolean) {
  return call<StrainSpecimenTree>('get_strain_specimen_tree', {
    strainId,
    includeDescendants: includeDescendants ?? false,
  });
}

export async function exportStrainPedigree(strainId: string, maxDepth?: number) {
  return call<PedigreeExport>('export_strain_pedigree', {
    strainId,
    maxDepth: maxDepth ?? null,
  });
}

// ── WP-32: Cryopreservation & LN2 Inventory ──────────────────────────────

export interface FrozenVial {
  id: string;
  specimen_id: string | null;
  species_id: string;
  species_code: string | null;
  species_name: string | null;
  passage_number: number;
  cumulative_pdl: number | null;
  vial_count: number;
  freeze_date: string;
  freeze_medium: string;
  location: string | null;
  location_freezer: string | null;
  location_tower: string | null;
  location_box: string | null;
  location_position: string | null;
  status: 'active' | 'depleted' | 'discarded';
  notes: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface ThawVialResult {
  updated_vial: FrozenVial;
  new_specimen_id: string;
  new_specimen_accession: string;
}

export async function createFrozenVial(request: {
  specimen_id?: string | null;
  species_id: string;
  passage_number: number;
  cumulative_pdl?: number | null;
  vial_count: number;
  freeze_date: string;
  freeze_medium: string;
  location_freezer?: string | null;
  location_tower?: string | null;
  location_box?: string | null;
  location_position?: string | null;
  notes?: string | null;
}) {
  return call<FrozenVial>('create_frozen_vial', { request });
}

export async function listFrozenVials(params?: {
  species_id?: string | null;
  specimen_id?: string | null;
  status?: string | null;
  location_freezer?: string | null;
}) {
  return call<FrozenVial[]>('list_frozen_vials', { params: params ?? null });
}

export async function getFrozenVial(id: string) {
  return call<FrozenVial>('get_frozen_vial', { id });
}

export async function thawVial(request: {
  vial_id: string;
  thaw_date: string;
  vials_to_thaw?: number | null;
  location?: string | null;
  notes?: string | null;
  employee_id?: string | null;
}) {
  return call<ThawVialResult>('thaw_vial', { request });
}

export async function discardFrozenVial(request: {
  vial_id: string;
  notes?: string | null;
}) {
  return call<FrozenVial>('discard_frozen_vial', { request });
}

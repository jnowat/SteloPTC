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

export interface ColonizationEntry {
  subculture_id: string;
  date: string;
  colonization_pct: number;
  passage_number: number;
  notes?: string | null;
}

export async function getColonizationHistory(specimenId: string): Promise<ColonizationEntry[]> {
  return call<ColonizationEntry[]>('get_colonization_history', { specimenId });
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

export async function getMycoplasmaStatus() {
  return call<any[]>('get_mycoplasma_status');
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

// ── WP-34: Cell-culture dashboard panels ─────────────────────────────────────

export interface VialLineSummary {
  species_id: string;
  species_code: string;
  species_name: string;
  /** Number of active (non-depleted, non-discarded) lots. */
  active_lots: number;
  /** Total vials across all active lots for this line. */
  total_vials: number;
  /** Vial count in the smallest active lot (low-stock risk indicator). */
  min_vials_in_lot: number;
}

export interface CultureMaintenanceAlert {
  specimen_id: string;
  accession_number: string;
  species_code: string;
  stage: string;
  stage_label: string;
  last_passage_date: string | null;
  days_since_passage: number | null;
}

/** Frozen vial inventory grouped by cell line, active lots only, ascending by total vials. */
export async function getVialSummaryByLine() {
  return call<VialLineSummary[]>('get_vial_summary_by_line');
}

/** Specimens in non-terminal profile stages not passaged in the last 7 days. */
export async function getCultureMaintenanceAlerts() {
  return call<CultureMaintenanceAlert[]>('get_culture_maintenance_alerts');
}

// ── WP-43: Fruiting records ───────────────────────────────────────────────────

export interface FruitingRecord {
  id: string;
  specimen_id: string;
  flush_number: number;
  harvest_date: string;
  fresh_weight_g: number | null;
  dry_weight_g: number | null;
  fruiting_temp_c: number | null;
  fruiting_rh_percent: number | null;
  fae_rate: number | null;
  light_hours_per_day: number | null;
  notes: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export async function createFruitingRecord(request: {
  specimen_id: string;
  flush_number: number;
  harvest_date: string;
  fresh_weight_g?: number;
  dry_weight_g?: number;
  fruiting_temp_c?: number;
  fruiting_rh_percent?: number;
  fae_rate?: number;
  light_hours_per_day?: number;
  notes?: string;
}) {
  return call<FruitingRecord>('create_fruiting_record', { request });
}

export async function listFruitingRecords(specimenId: string) {
  return call<FruitingRecord[]>('list_fruiting_records', { specimenId });
}

// ── Breeding programs (WP-47) ─────────────────────────────────────────────────

export interface BreedingProgram {
  id: string;
  name: string;
  goal: string | null;
  start_date: string | null;
  target_traits: string | null;
  founder_strain_ids: string | null;
  notes: string | null;
  created_at: string;
  created_by: string | null;
}

export interface BreedingRecord {
  id: string;
  program_id: string;
  strain_id: string;
  generation_number: number;
  selection_notes: string | null;
  fitness_score: number | null;
  selection_date: string | null;
  selected_by: string | null;
  notes: string | null;
  created_at: string;
}

export interface GenerationalSummary {
  generation_number: number;
  record_count: number;
  avg_fitness: number | null;
}

export async function createBreedingProgram(request: {
  name: string;
  goal?: string;
  start_date?: string;
  target_traits?: string;
  founder_strain_ids?: string;
  notes?: string;
}) {
  return call<BreedingProgram>('create_breeding_program', { request });
}

export async function listBreedingPrograms() {
  return call<BreedingProgram[]>('list_breeding_programs');
}

export async function getBreedingProgram(id: string) {
  return call<BreedingProgram>('get_breeding_program', { id });
}

export async function addBreedingRecord(request: {
  program_id: string;
  strain_id: string;
  generation_number: number;
  selection_notes?: string;
  fitness_score?: number;
  selection_date?: string;
  notes?: string;
}) {
  return call<BreedingRecord>('add_breeding_record', { request });
}

export async function listBreedingRecordsForProgram(programId: string) {
  return call<BreedingRecord[]>('list_breeding_records_for_program', { programId });
}

export async function listBreedingRecordsForStrain(strainId: string) {
  return call<BreedingRecord[]>('list_breeding_records_for_strain', { strainId });
}

export async function getGenerationalSummary(programId: string) {
  return call<GenerationalSummary[]>('get_generational_summary', { programId });
}

// WP-49: Provisional taxa & Darwin Core export

export interface TaxonMapping {
  id: string;
  provisional_taxon_id: string;
  accepted_taxon_id: string | null;
  accepted_ncbi_id: number | null;
  accepted_name: string | null;
  notes: string | null;
  mapped_by: string | null;
  mapped_at: string;
}

export interface DarwinCoreRecord {
  taxonID: string;
  scientificName: string;
  taxonRank: string;
  parentNameUsageID: string | null;
  taxonomicStatus: string;
  nameAccordingTo: string | null;
  remarks: string | null;
}

export interface DarwinCoreExport {
  record_count: number;
  records: DarwinCoreRecord[];
}

export async function createProvisionalTaxon(request: {
  rank: TaxonRank;
  name: string;
  parent_id?: string;
  provisional_notes?: string;
}) {
  return call<Taxon>('create_provisional_taxon', { request });
}

export async function listProvisionalTaxa() {
  return call<Taxon[]>('list_provisional_taxa');
}

export async function mapProvisionalTaxon(request: {
  provisional_taxon_id: string;
  accepted_taxon_id?: string;
  accepted_ncbi_id?: number;
  accepted_name?: string;
  notes?: string;
}) {
  return call<TaxonMapping>('map_provisional_taxon', { request });
}

export async function listTaxonMappings() {
  return call<TaxonMapping[]>('list_taxon_mappings');
}

export async function exportDarwinCore(rootId?: string) {
  return call<DarwinCoreExport>('export_darwin_core', { rootId: rootId ?? null });
}

// ---------------------------------------------------------------------------
// WP-50 — Backend configuration foundation
// ---------------------------------------------------------------------------
// SQLite remains the only backend actually serving reads/writes. These calls
// let an admin record an intended future backend and test PostgreSQL
// connectivity; neither reconnects the live app.

export interface BackendConfigInfo {
  backend_type: 'sqlite' | 'postgres';
  postgres_feature_compiled: boolean;
}

export async function getBackendConfig() {
  return call<BackendConfigInfo>('get_backend_config');
}

export async function setBackendType(backendType: 'sqlite' | 'postgres', connectionString?: string) {
  return call<void>('set_backend_type', {
    backendType,
    connectionString: connectionString ?? null,
  });
}

export async function testPostgresConnection(connectionString: string) {
  return call<string>('test_postgres_connection', { connectionString });
}

export async function bootstrapPostgresSchema(connectionString: string) {
  return call<string[]>('bootstrap_postgres_schema', { connectionString });
}

// ---------------------------------------------------------------------------
// WP-51 — LAN sync foundation
// ---------------------------------------------------------------------------
// Data-model and command surface only — there is no networking/transport
// layer yet. These calls reuse the existing audit hash chain for change
// detection; `applyIncomingChanges` records conflicts durably but does not
// yet write accepted changes back into specimens/subcultures/etc.

export interface SyncCursor {
  lineage_id: string;
  last_seen_chain_seq: number;
}

export interface ChangeRecord {
  lineage_id: string;
  chain_seq: number;
  entity_type: string;
  entity_id: string | null;
  action: string;
  old_value: string | null;
  new_value: string | null;
  details: string | null;
  prev_hash: string | null;
  entry_hash: string | null;
  created_at: string;
}

export interface ChangeSetResponse {
  changes: ChangeRecord[];
  has_more: boolean;
}

export interface SyncConflict {
  id: string;
  lineage_id: string;
  chain_seq: number;
  local_entry_hash: string | null;
  incoming_entry_hash: string | null;
  incoming_source_device_id: string | null;
  reason: string;
  resolved: boolean;
  resolved_by: string | null;
  resolved_at: string | null;
  detected_at: string;
}

export interface ApplyChangesResult {
  applied: number;
  skipped_duplicate: number;
  pending_manual_apply: number;
  conflicts: SyncConflict[];
}

export interface SyncPeer {
  id: string;
  device_id: string;
  device_name: string;
  last_seen_at: string | null;
  last_sync_at: string | null;
  created_at: string;
}

export interface SyncStatusResponse {
  lineages_tracked: number;
  max_chain_seq_overall: number;
  unresolved_conflicts: number;
  known_peers: number;
}

export async function getSyncStatus() {
  return call<SyncStatusResponse>('get_sync_status');
}

export async function getChangesSinceCursor(cursors: SyncCursor[], limit?: number) {
  return call<ChangeSetResponse>('get_changes_since_cursor', { cursors, limit: limit ?? null });
}

export async function applyIncomingChanges(changes: ChangeRecord[], sourceDeviceId: string) {
  return call<ApplyChangesResult>('apply_incoming_changes', {
    request: { changes, source_device_id: sourceDeviceId },
  });
}

export async function listSyncConflicts(unresolvedOnly?: boolean) {
  return call<SyncConflict[]>('list_sync_conflicts', { unresolvedOnly: unresolvedOnly ?? null });
}

export async function resolveSyncConflict(conflictId: string, resolutionNote: string) {
  return call<void>('resolve_sync_conflict', { conflictId, resolutionNote });
}

export async function registerSyncPeer(deviceId: string, deviceName: string) {
  return call<string>('register_sync_peer', { deviceId, deviceName });
}

export async function listSyncPeers() {
  return call<SyncPeer[]>('list_sync_peers');
}

// ---------------------------------------------------------------------------
// WP-55 — Field-level permissions
// ---------------------------------------------------------------------------

/** Sentinel value a masked field is replaced with. Matches db::permissions::RESTRICTED_MARKER. */
export const RESTRICTED_MARKER = '[RESTRICTED]';

export interface FieldPermission {
  id: string;
  role: 'admin' | 'supervisor' | 'tech' | 'guest';
  entity_type: string;
  field_name: string;
  visible: boolean;
}

export async function listFieldPermissions() {
  return call<FieldPermission[]>('list_field_permissions');
}

export async function setFieldPermission(role: string, entityType: string, fieldName: string, visible: boolean) {
  return call<void>('set_field_permission', {
    request: { role, entity_type: entityType, field_name: fieldName, visible },
  });
}

// ---------------------------------------------------------------------------
// WP-54 — Environmental sensor integration
// ---------------------------------------------------------------------------

export interface EnvironmentalReading {
  id: string;
  specimen_id: string | null;
  subculture_id: string | null;
  reading_type: string;
  value: number;
  unit: string | null;
  source: 'manual' | 'usb_serial' | 'bluetooth' | 'mqtt';
  recorded_at: string;
  notes: string | null;
  created_by: string | null;
  created_at: string;
}

export interface CreateEnvironmentalReadingRequest {
  specimen_id?: string;
  subculture_id?: string;
  reading_type: string;
  value: number;
  unit?: string;
  source?: string;
  recorded_at?: string;
  notes?: string;
}

export interface EnvironmentalAlert {
  specimen_id: string | null;
  reading_type: string;
  value: number;
  threshold_min: number | null;
  threshold_max: number | null;
  message: string;
  recorded_at: string;
}

export async function createEnvironmentalReading(request: CreateEnvironmentalReadingRequest) {
  return call<string>('create_environmental_reading', { request });
}

export async function ingestSensorPayload(
  source: string,
  rawPayload: string,
  specimenId?: string,
  subcultureId?: string,
) {
  return call<string[]>('ingest_sensor_payload', {
    specimenId: specimenId ?? null,
    subcultureId: subcultureId ?? null,
    source,
    rawPayload,
  });
}

export async function listEnvironmentalReadings(specimenId: string, limit?: number) {
  return call<EnvironmentalReading[]>('list_environmental_readings', { specimenId, limit: limit ?? null });
}

export async function getEnvironmentalAlerts() {
  return call<EnvironmentalAlert[]>('get_environmental_alerts');
}

// ---------------------------------------------------------------------------
// WP-52 — Email/desktop notifications
// ---------------------------------------------------------------------------

export interface NotificationPreference {
  id: string;
  user_id: string;
  channel: 'desktop' | 'email' | 'mobile_push';
  enabled: boolean;
  min_severity: 'normal' | 'high' | 'critical';
}

export interface SmtpConfig {
  host: string | null;
  port: number;
  username: string | null;
  password_set: boolean;
  from_address: string | null;
  use_tls: boolean;
}

export interface DispatchNotificationsResult {
  candidates_found: number;
  desktop_sent: number;
  email_sent: number;
  recipients_notified: number;
}

export async function getNotificationPreferences() {
  return call<NotificationPreference[]>('get_notification_preferences');
}

export async function setNotificationPreference(channel: string, enabled: boolean, minSeverity: string) {
  return call<void>('set_notification_preference', {
    request: { channel, enabled, min_severity: minSeverity },
  });
}

export async function getSmtpConfig() {
  return call<SmtpConfig>('get_smtp_config');
}

export async function setSmtpConfig(config: {
  host?: string;
  port: number;
  username?: string;
  password?: string;
  from_address?: string;
  use_tls: boolean;
}) {
  return call<void>('set_smtp_config', { request: config });
}

export async function sendTestDesktopNotification() {
  return call<void>('send_test_desktop_notification');
}

export async function sendTestEmail(toAddress: string) {
  return call<void>('send_test_email', { toAddress });
}

export async function dispatchDueNotificationsNow() {
  return call<DispatchNotificationsResult>('dispatch_due_notifications_now');
}

// ── WP-58: Analytics & reporting dashboards ─────────────────────────────────

export type AnalyticsTimeRange = '30d' | '90d' | '1y' | 'all';

export async function getSpecimenGrowthRate(timeRange: AnalyticsTimeRange) {
  return call<Array<{ bucket: string; value: number }>>('get_specimen_growth_rate', { timeRange });
}

export async function getSubcultureFrequencyTrend(timeRange: AnalyticsTimeRange, speciesId?: string) {
  return call<Array<{ bucket: string; value: number }>>('get_subculture_frequency_trend', { timeRange, speciesId: speciesId ?? null });
}

export async function getContaminationRateTrend(timeRange: AnalyticsTimeRange) {
  return call<Array<{ bucket: string; value: number }>>('get_contamination_rate_trend', { timeRange });
}

export async function getPassageSuccessRate(timeRange: AnalyticsTimeRange) {
  return call<{ total_passages: number; successful_passages: number; success_rate_pct: number; trend_delta_pct: number }>(
    'get_passage_success_rate', { timeRange },
  );
}

export async function getMediaBatchEfficiency(timeRange: AnalyticsTimeRange) {
  return call<Array<{ batch_id: string; name: string; specimens_supported: number; waste_rate_pct: number }>>(
    'get_media_batch_efficiency', { timeRange },
  );
}

export async function getStrainPerformance(speciesId: string) {
  return call<Array<{
    strain_id: string; strain_name: string; mean_health: number | null;
    total_specimens: number; avg_days_between_passages: number | null; best_performer_rate_pct: number;
  }>>('get_strain_performance', { speciesId });
}

export async function getCryoUtilization() {
  return call<Array<{ species_id: string; species_code: string; vials_active: number; vials_depleted_or_discarded: number; utilization_rate_pct: number }>>(
    'get_cryo_utilization',
  );
}

export async function getTechnicianActivity(timeRange: AnalyticsTimeRange) {
  return call<Array<{ user_id: string; display_name: string; passages_recorded: number; contamination_events: number }>>(
    'get_technician_activity', { timeRange },
  );
}

export async function getAnalyticsKpiSummary() {
  return call<{
    total_active_specimens: number; passages_this_week: number; contamination_rate_this_month_pct: number;
    pending_work_queue_items: number; passages_per_active_specimen: number;
    new_specimens_this_month: number; new_specimens_last_month: number;
  }>('get_analytics_kpi_summary');
}

export async function getAnalyticsPanelConfig() {
  return call<string>('get_analytics_panel_config');
}

export async function setAnalyticsPanelConfig(configJson: string) {
  return call<void>('set_analytics_panel_config', { configJson });
}

// ── WP-57: Interactive lab map ───────────────────────────────────────────────

export interface Location {
  id: string;
  name: string;
  description: string | null;
  floor_plan_image: string | null;
  floor_plan_x: number | null;
  floor_plan_y: number | null;
  created_at: string;
  updated_at: string;
}

export async function listLocations() {
  return call<Location[]>('list_locations');
}

export async function getLocation(id: string) {
  return call<Location>('get_location', { id });
}

export async function createLocation(request: {
  name: string; description?: string; floor_plan_image?: string; floor_plan_x?: number; floor_plan_y?: number;
}) {
  return call<Location>('create_location', { request });
}

export async function updateLocation(request: {
  id: string; name?: string; description?: string; floor_plan_image?: string; floor_plan_x?: number; floor_plan_y?: number;
}) {
  return call<Location>('update_location', { request });
}

export async function deleteLocation(id: string) {
  return call<void>('delete_location', { id });
}

export async function setSpecimenLocationPin(specimenId: string, locationId: string | null) {
  return call<void>('set_specimen_location_pin', { specimenId, locationId });
}

export async function getLocationMapData() {
  return call<Array<{
    location_id: string; name: string; floor_plan_x: number | null; floor_plan_y: number | null;
    specimen_count: number; contaminated_count: number; avg_age_days: number | null;
  }>>('get_location_map_data');
}

// ── WP-56: Local AI analysis ─────────────────────────────────────────────────

export interface AiSuggestion {
  id: string;
  entity_type: string;
  entity_id: string;
  kind: string;
  model_name: string;
  prompt: string;
  suggestion: string;
  status: string;
  created_by: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  created_at: string;
}

export interface AiConfig {
  provider: string; // 'ollama' | 'localai'
  base_url: string;
  text_model: string;
  vision_model: string;
}

export interface AiStatus {
  provider: string;
  base_url: string;
  reachable: boolean;
  models: string[];
  text_model: string;
  vision_model: string;
  text_model_installed: boolean;
  vision_model_installed: boolean;
  error: string | null;
}

export async function getAiConfig() {
  return call<AiConfig>('get_ai_config');
}

export async function setAiConfig(provider: string, baseUrl: string, textModel: string, visionModel: string) {
  return call<void>('set_ai_config', { provider, baseUrl, textModel, visionModel });
}

/** Live reachability probe: is the local AI runtime up, and which models does it have? */
export async function getAiStatus() {
  return call<AiStatus>('get_ai_status');
}

export async function summarizeNotes(entityType: 'specimen' | 'subculture', entityId: string) {
  return call<AiSuggestion>('summarize_notes', { request: { entity_type: entityType, entity_id: entityId } });
}

export async function suggestPassageComment(specimenId: string) {
  return call<AiSuggestion>('suggest_passage_comment', { specimenId });
}

export async function analyzePhotoForContamination(attachmentId: string) {
  return call<AiSuggestion>('analyze_photo_for_contamination', { request: { attachment_id: attachmentId } });
}

export async function listAiSuggestions(entityType: string, entityId: string) {
  return call<AiSuggestion[]>('list_ai_suggestions', { entityType, entityId });
}

export async function approveAiSuggestion(suggestionId: string) {
  return call<void>('approve_ai_suggestion', { suggestionId });
}

export async function rejectAiSuggestion(suggestionId: string) {
  return call<void>('reject_ai_suggestion', { suggestionId });
}

// ── WP-59: Cloud backup & multi-device sync ──────────────────────────────────

export interface BackupTargetSummary {
  id: string;
  name: string;
  target_type: string;
  schedule_cron: string | null;
  last_backup_at: string | null;
  last_backup_size_bytes: number | null;
  last_backup_size_display: string | null;
  last_status: string | null;
  last_error: string | null;
  is_enabled: boolean;
}

export async function listBackupTargets() {
  return call<BackupTargetSummary[]>('list_backup_targets');
}

export async function createBackupTarget(request: {
  name: string; targetType: string; passphrase: string; bucketOrPath: string;
  endpoint?: string; accessKey?: string; secretKey?: string; scheduleCron?: string;
}) {
  return call<BackupTargetSummary>('create_backup_target', {
    name: request.name, targetType: request.targetType, passphrase: request.passphrase,
    bucketOrPath: request.bucketOrPath, endpoint: request.endpoint ?? null,
    accessKey: request.accessKey ?? null, secretKey: request.secretKey ?? null,
    scheduleCron: request.scheduleCron ?? null,
  });
}

export async function deleteBackupTarget(id: string) {
  return call<void>('delete_backup_target', { id });
}

export async function cloudBackup(targetId: string, passphrase: string) {
  return call<{ ok: boolean; backup_id: string; size_bytes: number; duration_ms: number; merkle_root_included: boolean }>(
    'cloud_backup', { targetId, passphrase },
  );
}

export async function restoreFromCloud(targetId: string, passphrase: string, backupFileName: string) {
  return call<string>('restore_from_cloud', { targetId, passphrase, backupFileName });
}

export async function reconcileCloudSync(targetId: string, passphrase: string, deviceId: string) {
  return call<{ segments_published: boolean; peer_segments_found: number; new_changes: number; duplicates: number; conflicts_recorded: number }>(
    'reconcile_cloud_sync', { targetId, passphrase, deviceId },
  );
}

// ── WP-60: Regulatory compliance export modules ──────────────────────────────

export async function getSigningPublicKey() {
  return call<string>('get_signing_public_key');
}

export async function exportFdaPart11Bundle(fromDate: string, toDate: string, labName: string) {
  return call<{ ok: boolean; file_path: string; size_bytes: number }>('export_fda_part11_bundle', { fromDate, toDate, labName });
}

export async function exportUsdaPermit(specimenIds: string[], authorizedScientist: string) {
  return call<{ ok: boolean; file_path: string; size_bytes: number }>('export_usda_permit', { specimenIds, authorizedScientist });
}

export async function exportCitesDossier(rootSpecimenId: string, citesAppendix: string) {
  return call<{ ok: boolean; file_path: string; size_bytes: number }>('export_cites_dossier', { rootSpecimenId, citesAppendix });
}

// ── WP-61: Plugin / extension system ─────────────────────────────────────────

export interface InstalledPlugin {
  id: string;
  plugin_name: string;
  version: string;
  profile: string | null;
  vocabulary_seeded: boolean;
  installed_at: string;
}

export async function listInstalledPlugins() {
  return call<InstalledPlugin[]>('list_installed_plugins');
}

export async function validatePluginManifest(manifestJson: string) {
  return call<any>('validate_plugin_manifest', { manifestJson });
}

export async function installPlugin(manifestJson: string) {
  return call<InstalledPlugin>('install_plugin', { manifestJson });
}

export async function installPluginFromZip(zipB64: string) {
  return call<InstalledPlugin>('install_plugin_from_zip', { zipB64 });
}

export async function uninstallPlugin(pluginId: string) {
  return call<void>('uninstall_plugin', { pluginId });
}

// ── WP-63: Performance & scalability — cursor pagination + pedigree depth ────

export async function listAuditEntriesCursor(lineageId: string, afterSeq: number | null, limit: number) {
  return call<{ items: any[]; next_cursor: number | null; has_more: boolean }>(
    'list_audit_entries_cursor', { lineageId, afterSeq, limit },
  );
}

export async function getPedigreeMaxDepth() {
  return call<number>('get_pedigree_max_depth');
}

export async function setPedigreeMaxDepth(maxDepth: number) {
  return call<number>('set_pedigree_max_depth', { maxDepth });
}

// ── WP-64: Taxon chain re-anchoring ──────────────────────────────────────────

export async function reanchorTaxonChainDryRun(taxonId: string) {
  return call<{ affected_taxa: number; affected_species: number; affected_strains: number; affected_specimens: number }>(
    'reanchor_taxon_chain_dry_run', { taxonId },
  );
}

export async function reanchorTaxonChain(taxonId: string, reason: string) {
  return call<{
    ok: boolean; affected_taxa: number; affected_species: number;
    affected_strains: number; affected_specimens: number; reanchor_event_id: string;
  }>('reanchor_taxon_chain', { taxonId, reason });
}

// ── WP-66: Trust Layer Phase 2 — on-chain anchoring (Dogecoin OP_RETURN) ──────

export interface CheckpointAnchor {
  id: string;
  checkpoint_id: string;
  chain_name: string;
  merkle_root: string;
  op_return_hex: string;
  txid: string | null;
  status: 'prepared' | 'submitted' | 'confirmed';
  verified_at: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface AnchorPayloadPreview {
  merkle_root: string;
  payload_hex: string;
  op_return_script_hex: string;
  chain_name: string;
  marker: string;
  version: number;
  byte_size: number;
}

export interface AnchorVerifyResult {
  anchor_id: string;
  ok: boolean;
  expected_root: string;
  found_root: string | null;
  message: string;
}

export async function previewCheckpointAnchorPayload(checkpointId: string, chainName?: string) {
  return call<AnchorPayloadPreview>('preview_checkpoint_anchor_payload', { checkpointId, chainName });
}

export async function prepareCheckpointAnchor(checkpointId: string, chainName?: string) {
  return call<CheckpointAnchor>('prepare_checkpoint_anchor', { checkpointId, chainName });
}

export async function recordCheckpointAnchor(anchorId: string, txid: string) {
  return call<CheckpointAnchor>('record_checkpoint_anchor', { anchorId, txid });
}

export async function verifyCheckpointAnchor(anchorId: string, opReturnHex: string) {
  return call<AnchorVerifyResult>('verify_checkpoint_anchor', { anchorId, opReturnHex });
}

export async function listCheckpointAnchors(checkpointId?: string) {
  return call<CheckpointAnchor[]>('list_checkpoint_anchors', { checkpointId });
}

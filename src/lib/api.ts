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
    if (msg.includes('Session expired') || msg.includes('invalid')) {
      clearAuth();
    }
    throw new Error(msg);
  }
}

// Auth (login doesn't need token)
export async function login(username: string, password: string) {
  try {
    return await invoke<{ token: string; user: any }>('login', { username, password });
  } catch (e: unknown) {
    const msg = typeof e === 'string' ? e : (e instanceof Error ? e.message : 'Login failed');
    throw new Error(msg);
  }
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
export async function listSubcultures(specimenId: string) {
  return call<any[]>('list_subcultures', { specimenId });
}

export async function createSubculture(request: any) {
  return call<any>('create_subculture', { request });
}

export async function updateSubculture(request: any) {
  return call<void>('update_subculture', { request });
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

export async function dismissReminder(id: string, snooze: boolean) {
  return call<void>('dismiss_reminder', { id, snooze });
}

// Compliance
export async function listComplianceRecords(specimenId?: string) {
  return call<any[]>('list_compliance_records', { specimenId: specimenId ?? null });
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

// Export
export async function exportSpecimensCsv() {
  return call<string>('export_specimens_csv');
}

export async function exportSpecimensJson() {
  return call<string>('export_specimens_json');
}

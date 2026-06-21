import { writable, get } from 'svelte/store';
import { getLabProfile } from './api';

export type LabProfile = 'plant_tissue_culture' | 'cell_culture' | 'mycology';

/** Current active lab profile. Defaults to plant_tissue_culture until loaded. */
export const labProfile = writable<LabProfile>('plant_tissue_culture');

export const LAB_PROFILE_LABELS: Record<LabProfile, string> = {
  plant_tissue_culture: 'Plant Tissue Culture',
  cell_culture: 'Cell Culture',
  mycology: 'Mycology',
};

/** Fetch the lab profile from the backend and update the store. */
export async function loadLabProfile(): Promise<void> {
  try {
    const p = await getLabProfile();
    labProfile.set(p as LabProfile);
  } catch {
    // Non-fatal — store retains default 'plant_tissue_culture'
  }
}

/** Return the current profile value synchronously (reads the store). */
export function currentLabProfile(): LabProfile {
  return get(labProfile);
}

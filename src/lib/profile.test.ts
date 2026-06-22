import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { labProfile, LAB_PROFILE_LABELS, currentLabProfile, type LabProfile } from './profile';

describe('labProfile store', () => {
  beforeEach(() => {
    labProfile.set('plant_tissue_culture');
  });

  it('defaults to plant_tissue_culture', () => {
    expect(get(labProfile)).toBe('plant_tissue_culture');
  });

  it('updates reactively when set', () => {
    labProfile.set('mycology');
    expect(get(labProfile)).toBe('mycology');
  });

  it('currentLabProfile() returns the current store value synchronously', () => {
    labProfile.set('cell_culture');
    expect(currentLabProfile()).toBe('cell_culture');
  });

  it('switching profile is reflected immediately', () => {
    const before: LabProfile = get(labProfile);
    labProfile.set('mycology');
    const after: LabProfile = get(labProfile);
    expect(before).toBe('plant_tissue_culture');
    expect(after).toBe('mycology');
  });
});

describe('LAB_PROFILE_LABELS', () => {
  it('has a label for every profile', () => {
    const profiles: LabProfile[] = ['plant_tissue_culture', 'cell_culture', 'mycology'];
    for (const p of profiles) {
      expect(LAB_PROFILE_LABELS[p]).toBeTruthy();
    }
  });

  it('plant_tissue_culture label is human-readable', () => {
    expect(LAB_PROFILE_LABELS['plant_tissue_culture']).toBe('Plant Tissue Culture');
  });
});

import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  labProfile,
  LAB_PROFILE_LABELS,
  currentLabProfile,
  PROFILE_DOMAIN,
  DOMAIN_MANIFESTS,
  activeDomainManifest,
  type LabProfile,
  type LabDomain,
} from './profile';

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

describe('PROFILE_DOMAIN', () => {
  it('maps every profile to a domain', () => {
    const profiles: LabProfile[] = ['plant_tissue_culture', 'cell_culture', 'mycology'];
    for (const p of profiles) {
      expect(PROFILE_DOMAIN[p]).toBeTruthy();
    }
  });

  it('plant_tissue_culture maps to Plantae', () => {
    expect(PROFILE_DOMAIN['plant_tissue_culture']).toBe('Plantae');
  });

  it('cell_culture maps to Animalia', () => {
    expect(PROFILE_DOMAIN['cell_culture']).toBe('Animalia');
  });

  it('mycology maps to Fungi', () => {
    expect(PROFILE_DOMAIN['mycology']).toBe('Fungi');
  });
});

describe('DOMAIN_MANIFESTS', () => {
  const domains: LabDomain[] = ['Plantae', 'Animalia', 'Fungi'];

  it('has a manifest for every domain', () => {
    for (const d of domains) {
      expect(DOMAIN_MANIFESTS[d]).toBeDefined();
    }
  });

  it('every manifest has a non-empty rankOrder', () => {
    for (const d of domains) {
      expect(DOMAIN_MANIFESTS[d].rankOrder.length).toBeGreaterThan(0);
    }
  });

  it('every manifest rankOrder contains genus and species', () => {
    for (const d of domains) {
      expect(DOMAIN_MANIFESTS[d].rankOrder).toContain('genus');
      expect(DOMAIN_MANIFESTS[d].rankOrder).toContain('species');
    }
  });

  it('every manifest has strainTypeLabels', () => {
    for (const d of domains) {
      expect(Object.keys(DOMAIN_MANIFESTS[d].strainTypeLabels).length).toBeGreaterThan(0);
    }
  });

  it('every manifest has confirmationMethodLabels', () => {
    for (const d of domains) {
      expect(Object.keys(DOMAIN_MANIFESTS[d].confirmationMethodLabels).length).toBeGreaterThan(0);
    }
  });

  it('Plantae manifest has cultivar strain type', () => {
    expect(DOMAIN_MANIFESTS['Plantae'].strainTypeLabels['cultivar']).toBe('Cultivar');
  });

  it('Animalia manifest has cell_line strain type', () => {
    expect(DOMAIN_MANIFESTS['Animalia'].strainTypeLabels['cell_line']).toBe('Cell Line');
  });

  it('Fungi manifest has wild_type strain type', () => {
    expect(DOMAIN_MANIFESTS['Fungi'].strainTypeLabels['wild_type']).toBe('Wild Type');
  });
});

describe('activeDomainManifest', () => {
  beforeEach(() => {
    labProfile.set('plant_tissue_culture');
  });

  it('returns Plantae manifest for plant_tissue_culture', () => {
    expect(activeDomainManifest()).toBe(DOMAIN_MANIFESTS['Plantae']);
  });

  it('returns Animalia manifest for cell_culture', () => {
    labProfile.set('cell_culture');
    expect(activeDomainManifest()).toBe(DOMAIN_MANIFESTS['Animalia']);
  });

  it('returns Fungi manifest for mycology', () => {
    labProfile.set('mycology');
    expect(activeDomainManifest()).toBe(DOMAIN_MANIFESTS['Fungi']);
  });
});

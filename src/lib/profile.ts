import { writable, get } from 'svelte/store';
import { getLabProfile } from './api';

export type LabProfile = 'plant_tissue_culture' | 'cell_culture' | 'mycology';

/** Biological domain (kingdom-level grouping) for a lab profile. */
export type LabDomain = 'Plantae' | 'Animalia' | 'Fungi';

/** Per-domain UI manifest: rank order, strain type labels, confirmation method labels. */
export interface DomainManifest {
  /** Ordered taxonomic ranks shown in the Taxonomy Navigator. */
  rankOrder: string[];
  /** Display labels for strain_type values relevant to this domain. */
  strainTypeLabels: Record<string, string>;
  /** Display labels for confirmation_basis values relevant to this domain. */
  confirmationMethodLabels: Record<string, string>;
}

/** Current active lab profile. Defaults to plant_tissue_culture until loaded. */
export const labProfile = writable<LabProfile>('plant_tissue_culture');

export const LAB_PROFILE_LABELS: Record<LabProfile, string> = {
  plant_tissue_culture: 'Plant Tissue Culture',
  cell_culture: 'Cell Culture',
  mycology: 'Mycology',
};

/** Maps each lab profile to its biological domain. */
export const PROFILE_DOMAIN: Record<LabProfile, LabDomain> = {
  plant_tissue_culture: 'Plantae',
  cell_culture: 'Animalia',
  mycology: 'Fungi',
};

/** Domain-specific UI manifests for rank navigation, strain types, and confirmation methods. */
export const DOMAIN_MANIFESTS: Record<LabDomain, DomainManifest> = {
  Plantae: {
    rankOrder: ['kingdom', 'phylum', 'class', 'order', 'family', 'genus', 'species'],
    strainTypeLabels: {
      cultivar: 'Cultivar',
      accession: 'Accession',
      ecotype: 'Ecotype',
      hybrid: 'Hybrid',
      landrace: 'Landrace',
    },
    confirmationMethodLabels: {
      morphological: 'Morphological',
      molecular: 'Molecular (PCR/Sequencing)',
      isozyme: 'Isozyme',
      visual: 'Visual Observation',
    },
  },
  Animalia: {
    rankOrder: ['kingdom', 'phylum', 'class', 'order', 'family', 'genus', 'species'],
    strainTypeLabels: {
      cell_line: 'Cell Line',
      primary: 'Primary Culture',
      immortalized: 'Immortalized',
      transformed: 'Transformed',
    },
    confirmationMethodLabels: {
      str_profiling: 'STR Profiling',
      karyotyping: 'Karyotyping',
      morphological: 'Morphological',
      flow_cytometry: 'Flow Cytometry',
    },
  },
  Fungi: {
    rankOrder: ['kingdom', 'phylum', 'class', 'order', 'family', 'genus', 'species'],
    strainTypeLabels: {
      wild_type: 'Wild Type',
      cultivated: 'Cultivated',
      hybrid: 'Hybrid',
      mutant: 'Mutant',
    },
    confirmationMethodLabels: {
      morphological: 'Morphological',
      molecular: 'Molecular (ITS Sequencing)',
      cultural: 'Cultural Characteristics',
      mating: 'Mating Tests',
    },
  },
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

/** Return the domain manifest for the currently active lab profile. */
export function activeDomainManifest(): DomainManifest {
  return DOMAIN_MANIFESTS[PROFILE_DOMAIN[currentLabProfile()]];
}

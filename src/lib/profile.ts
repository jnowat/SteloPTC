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

/**
 * Mycology culture-origin vocabulary. Single source of truth for BOTH the
 * SpecimenForm input `<select>` and the SpecimenDetail badge, so the two can
 * never drift apart (the exact class of input-vs-display label mismatch called
 * out in skills.md §7). `badge` is the CSS class for the detail-view chip.
 *
 * The keys MUST stay in lock-step with the `origin_type` CHECK constraint in
 * migration 029 (`multi_spore`/`isolated_dikaryon`/`tissue_clone`) — adding a
 * value here without the matching migration would let the UI submit a value the
 * database rejects.
 */
export interface OriginTypeMeta {
  label: string;
  badge: string;
}
export const ORIGIN_TYPE_META: Record<string, OriginTypeMeta> = {
  multi_spore: { label: 'Multi-Spore', badge: 'badge-blue' },
  isolated_dikaryon: { label: 'Isolated Dikaryon', badge: 'badge-purple' },
  tissue_clone: { label: 'Tissue Clone', badge: 'badge-green' },
};

/**
 * Mycology contamination categories for a passage/subculture. Single source of
 * truth for the contaminant `<select>` (and any future display of it). The
 * `contaminant_type` column has no DB CHECK constraint, so this list is the de
 * facto vocabulary — keep it here, not inline in a component.
 */
export const CONTAMINANT_TYPE_LABELS: Record<string, string> = {
  trich: 'Trichoderma (Trich)',
  wet_rot: 'Wet Rot / Bacterial',
  cobweb: 'Cobweb Mold',
  pin_mold: 'Pin Mold (Mucor / Rhizopus)',
  mycelium_abort: 'Mycelium Abort',
  other: 'Other',
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

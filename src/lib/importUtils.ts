// Pure import validation helpers. Extracted from ImportManager.svelte so they
// can be unit-tested without a browser or XLSX dependency.

export const REQUIRED_SHEET_NAMES = [
  'Specimens',
  'Subcultures',
  'Media Batches',
  'Prepared Solutions',
  'Inventory',
  'Compliance',
] as const;

/** Returns the names of any required sheets that are absent from the workbook. */
export function findMissingSheets(workbookSheetNames: string[]): string[] {
  return REQUIRED_SHEET_NAMES.filter(n => !workbookSheetNames.includes(n));
}

/**
 * Tolerant parsing of whatever an operator actually has in their clipboard when
 * they go to import NCBI taxonomy.
 *
 * The import screen used to accept exactly one thing: a JSON array of objects
 * with `ncbi_taxon_id` / `name` / `rank`. Nobody has that. What people have is
 * an E-utilities response, a chunk of a taxdump file, a spreadsheet column, or
 * a list of taxids they copied out of a browser tab. This module recognises all
 * of those and normalises them into the one shape the backend command takes, so
 * the "leeway" lives here rather than in the operator's text editor.
 *
 * Everything is offline and dependency-free: SteloPTC is a local-first app with
 * no HTTP client in the backend and no network permissions in its Tauri
 * capabilities, so this never fetches. For the inputs that genuinely cannot be
 * resolved without NCBI (a bare list of names has no taxids; a bare list of
 * taxids has no names), it builds the E-utilities URL the operator can open
 * themselves and paste the answer back.
 */

/** A taxon record in the shape `import_ncbi_taxonomy` expects. */
export interface ParsedTaxon {
  ncbi_taxon_id: number;
  name: string;
  rank: string;
  parent_ncbi_id: number | null;
  /** Short note on where this row came from, shown in the preview table. */
  source: string;
}

/** A line the parser understood the shape of but could not use. */
export interface ParseIssue {
  line: string;
  reason: string;
}

export type NcbiInputFormat =
  | 'empty'
  | 'record-json'
  | 'esummary-json'
  | 'efetch-xml'
  | 'taxdump'
  | 'delimited'
  | 'taxid-list'
  | 'name-list'
  | 'unknown';

export interface ParseResult {
  format: NcbiInputFormat;
  /** Human-readable name of the detected format, for the UI badge. */
  formatLabel: string;
  records: ParsedTaxon[];
  issues: ParseIssue[];
  /** Guidance that is not an error — e.g. "these need a round trip to NCBI". */
  notes: string[];
  /** Set when the input names things NCBI must resolve; the UI offers this URL. */
  lookupUrl?: string;
}

/** The six ranks the taxonomy backbone stores. Anything else is Species Registry territory. */
export const BACKBONE_RANKS = ['kingdom', 'phylum', 'class', 'order', 'family', 'genus'] as const;

/**
 * Ranks the backend's `normalize_ncbi_rank` accepts, including the NCBI spellings
 * it folds. Kept in sync with `db::queries::normalize_ncbi_rank` — a rank this
 * list accepts but the backend rejects would show a row as importable in the
 * preview and then silently drop it.
 */
const ACCEPTED_RANKS = new Set([
  'kingdom', 'superkingdom',
  'phylum', 'division',
  'class',
  'order',
  'family',
  'genus',
]);

export function isBackboneRank(rank: string): boolean {
  return ACCEPTED_RANKS.has(rank.trim().toLowerCase());
}

const FORMAT_LABELS: Record<NcbiInputFormat, string> = {
  'empty': 'Nothing pasted yet',
  'record-json': 'JSON taxon records',
  'esummary-json': 'NCBI E-utilities esummary (JSON)',
  'efetch-xml': 'NCBI E-utilities efetch (XML)',
  'taxdump': 'NCBI taxdump (nodes.dmp / names.dmp)',
  'delimited': 'Table (CSV / TSV)',
  'taxid-list': 'List of taxon IDs',
  'name-list': 'List of scientific names',
  'unknown': 'Unrecognised',
};

// ── Field aliases ────────────────────────────────────────────────────────────
//
// The same three facts appear under different keys depending on which NCBI
// endpoint or export produced the data. Matching on a normalised key (lowercased,
// punctuation stripped) collapses `TaxId`, `tax_id`, `taxid` and `Tax ID` to one
// case instead of needing an entry for each spelling.

const ID_KEYS = ['ncbitaxonid', 'taxid', 'uid', 'id', 'ncbiid', 'taxonid'];
const NAME_KEYS = ['name', 'scientificname', 'sciname', 'taxonname', 'nametxt'];
const RANK_KEYS = ['rank', 'taxrank', 'taxonrank'];
const PARENT_KEYS = ['parentncbiid', 'parenttaxid', 'parent', 'parentid', 'parentncbitaxonid'];

function normalizeKey(key: string): string {
  return key.toLowerCase().replace(/[^a-z0-9]/g, '');
}

function pick(obj: Record<string, unknown>, aliases: string[]): unknown {
  const byNormalized = new Map<string, unknown>();
  for (const [k, v] of Object.entries(obj)) {
    const nk = normalizeKey(k);
    // First spelling wins so an exact `name` is not shadowed by a later alias.
    if (!byNormalized.has(nk)) byNormalized.set(nk, v);
  }
  for (const alias of aliases) {
    if (byNormalized.has(alias)) {
      const value = byNormalized.get(alias);
      if (value !== undefined && value !== null && value !== '') return value;
    }
  }
  return undefined;
}

function toId(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return Math.trunc(value);
  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (/^\d+$/.test(trimmed)) return parseInt(trimmed, 10);
  }
  return null;
}

function toText(value: unknown): string {
  return typeof value === 'string' ? value.trim() : value == null ? '' : String(value).trim();
}

// ── Format detection ─────────────────────────────────────────────────────────

export function detectFormat(input: string): NcbiInputFormat {
  const text = input.trim();
  if (!text) return 'empty';
  if (text.startsWith('<')) return 'efetch-xml';

  if (text.startsWith('{') || text.startsWith('[')) {
    try {
      const parsed = JSON.parse(text);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed) && 'result' in parsed) {
        return 'esummary-json';
      }
      return 'record-json';
    } catch {
      // Malformed JSON is still JSON in intent — reporting a parse error is far
      // more useful than falling through and calling it a list of names.
      return 'record-json';
    }
  }

  // taxdump columns are separated by the literal three-character sequence
  // tab-pipe-tab, which nothing else in this list uses.
  if (text.includes('\t|\t') || /\|\s*$/m.test(text) && text.includes('|')) return 'taxdump';

  const lines = text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  if (lines.length === 0) return 'empty';

  const firstLine = lines[0];
  const hasSeparator = firstLine.includes('\t') || firstLine.includes(',');
  if (hasSeparator) {
    const cells = splitDelimited(firstLine).map((c) => normalizeKey(c));
    const looksLikeHeader = cells.some((c) => ID_KEYS.includes(c) || NAME_KEYS.includes(c) || RANK_KEYS.includes(c));
    if (looksLikeHeader || lines.length > 1) return 'delimited';
  }

  if (lines.every((l) => /^\d+$/.test(l))) return 'taxid-list';
  // A scientific name is letters, possibly several words, possibly hyphenated.
  if (lines.every((l) => /^[A-Za-z][A-Za-z .'\-]*$/.test(l))) return 'name-list';

  return 'unknown';
}

function splitDelimited(line: string): string[] {
  if (line.includes('\t')) return line.split('\t').map((c) => c.trim());
  return line.split(',').map((c) => c.trim().replace(/^"(.*)"$/, '$1'));
}

// ── The individual parsers ───────────────────────────────────────────────────

function parseRecordObject(raw: unknown, source: string, issues: ParseIssue[]): ParsedTaxon | null {
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) {
    issues.push({ line: String(raw).slice(0, 120), reason: 'not a taxon object' });
    return null;
  }
  const obj = raw as Record<string, unknown>;
  const id = toId(pick(obj, ID_KEYS));
  const name = toText(pick(obj, NAME_KEYS));
  const rank = toText(pick(obj, RANK_KEYS));

  const missing: string[] = [];
  if (id === null) missing.push('taxon ID');
  if (!name) missing.push('name');
  if (!rank) missing.push('rank');
  if (missing.length > 0) {
    issues.push({
      line: JSON.stringify(obj).slice(0, 120),
      reason: `missing ${missing.join(', ')}`,
    });
    return null;
  }

  return {
    ncbi_taxon_id: id as number,
    name,
    rank,
    parent_ncbi_id: toId(pick(obj, PARENT_KEYS)),
    source,
  };
}

function parseRecordJson(text: string, issues: ParseIssue[]): ParsedTaxon[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e: any) {
    issues.push({ line: text.slice(0, 120), reason: `JSON parse error: ${e.message}` });
    return [];
  }
  const arr = Array.isArray(parsed) ? parsed : [parsed];
  const out: ParsedTaxon[] = [];
  for (const item of arr) {
    const rec = parseRecordObject(item, 'JSON record', issues);
    if (rec) out.push(rec);
  }
  return out;
}

/**
 * NCBI esummary v2.0 JSON: `{ result: { uids: ["4932"], "4932": { … } } }`.
 * The `uids` array is authoritative for ordering; the sibling keys hold the
 * records. Older responses omit `uids`, so fall back to every non-`uids` key.
 */
function parseEsummary(text: string, issues: ParseIssue[]): ParsedTaxon[] {
  let parsed: any;
  try {
    parsed = JSON.parse(text);
  } catch (e: any) {
    issues.push({ line: text.slice(0, 120), reason: `JSON parse error: ${e.message}` });
    return [];
  }
  const result = parsed?.result;
  if (!result || typeof result !== 'object') {
    issues.push({ line: 'result', reason: 'esummary response has no "result" object' });
    return [];
  }
  const uids: string[] = Array.isArray(result.uids)
    ? result.uids.map(String)
    : Object.keys(result).filter((k) => k !== 'uids');

  const out: ParsedTaxon[] = [];
  for (const uid of uids) {
    const entry = result[uid];
    if (!entry || typeof entry !== 'object') {
      issues.push({ line: uid, reason: 'no record for this uid' });
      continue;
    }
    // esummary omits the uid inside the record on some endpoints; the key is it.
    const withUid = { uid, ...entry };
    const rec = parseRecordObject(withUid, `esummary #${uid}`, issues);
    if (rec) out.push(rec);
  }
  return out;
}

/**
 * NCBI efetch XML. Each `<Taxon>` carries `TaxId` / `ScientificName` / `Rank` /
 * `ParentTaxId`, and usually a `<LineageEx>` holding every ancestor in order
 * from root to immediate parent.
 *
 * Expanding `LineageEx` is the point of supporting this format: one efetch of a
 * single species yields the whole Kingdom → … → Genus backbone above it, which
 * is exactly what the taxonomy tree needs and what an operator would otherwise
 * have to enter by hand. Lineage entries carry no ParentTaxId of their own, so
 * parents are chained from their document order.
 */
function parseEfetchXml(text: string, issues: ParseIssue[]): ParsedTaxon[] {
  if (typeof DOMParser === 'undefined') {
    issues.push({ line: 'XML', reason: 'XML parsing is unavailable in this environment' });
    return [];
  }
  const doc = new DOMParser().parseFromString(text, 'application/xml');
  if (doc.getElementsByTagName('parsererror').length > 0) {
    issues.push({ line: text.slice(0, 120), reason: 'malformed XML' });
    return [];
  }

  const childText = (el: Element, tag: string): string => {
    // Direct children only: a <Taxon> inside <LineageEx> also has a <TaxId>, and
    // getElementsByTagName would reach into it and return the ancestor's value.
    for (const child of Array.from(el.children)) {
      if (child.tagName === tag) return (child.textContent ?? '').trim();
    }
    return '';
  };

  const out: ParsedTaxon[] = [];
  const emitted = new Set<number>();
  const push = (rec: ParsedTaxon) => {
    if (emitted.has(rec.ncbi_taxon_id)) return;
    emitted.add(rec.ncbi_taxon_id);
    out.push(rec);
  };

  // Top-level <Taxon> elements are those not nested inside a <LineageEx>.
  const allTaxa = Array.from(doc.getElementsByTagName('Taxon'));
  const topLevel = allTaxa.filter((t) => t.parentElement?.tagName !== 'LineageEx');

  for (const taxon of topLevel) {
    const lineageEx = Array.from(taxon.children).find((c) => c.tagName === 'LineageEx');
    let previousId: number | null = null;

    if (lineageEx) {
      for (const ancestor of Array.from(lineageEx.children)) {
        if (ancestor.tagName !== 'Taxon') continue;
        const id = toId(childText(ancestor, 'TaxId'));
        const name = childText(ancestor, 'ScientificName');
        const rank = childText(ancestor, 'Rank');
        if (id === null || !name) continue;
        push({
          ncbi_taxon_id: id,
          name,
          rank: rank || 'no rank',
          parent_ncbi_id: previousId,
          source: 'efetch lineage',
        });
        previousId = id;
      }
    }

    const id = toId(childText(taxon, 'TaxId'));
    const name = childText(taxon, 'ScientificName');
    const rank = childText(taxon, 'Rank');
    if (id === null || !name) {
      issues.push({ line: name || 'Taxon', reason: 'Taxon element has no TaxId or ScientificName' });
      continue;
    }
    push({
      ncbi_taxon_id: id,
      name,
      rank: rank || 'no rank',
      parent_ncbi_id: toId(childText(taxon, 'ParentTaxId')) ?? previousId,
      source: 'efetch record',
    });
  }

  return out;
}

/**
 * NCBI taxdump files. `nodes.dmp` rows are
 * `tax_id | parent_tax_id | rank | …` and `names.dmp` rows are
 * `tax_id | name_txt | unique_name | name_class |`.
 *
 * Both are accepted, in either order and concatenated, because that is how they
 * arrive when someone greps a taxdump. nodes.dmp supplies id/parent/rank and
 * names.dmp supplies the name; a row is only emitted once it has both, and the
 * rest are reported rather than dropped.
 */
function parseTaxdump(text: string, issues: ParseIssue[]): ParsedTaxon[] {
  interface Partial { id: number; rank?: string; parent?: number | null; name?: string }
  const byId = new Map<number, Partial>();

  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;
    // Strip the trailing "|" every dmp row ends with, then split on the pipes.
    const cells = line.replace(/\|\s*$/, '').split('|').map((c) => c.trim());
    if (cells.length < 2) continue;

    const id = toId(cells[0]);
    if (id === null) {
      issues.push({ line: line.slice(0, 120), reason: 'first column is not a taxon ID' });
      continue;
    }
    const entry = byId.get(id) ?? { id };

    // names.dmp is identified by its name class in column 4.
    const nameClass = cells[3]?.toLowerCase();
    if (nameClass) {
      // Only the scientific name is a taxon's name; synonyms and common names
      // would each overwrite it with a different string.
      if (nameClass === 'scientific name') entry.name = cells[1];
    } else if (cells.length >= 3) {
      // nodes.dmp: id | parent | rank | …
      entry.parent = toId(cells[1]);
      entry.rank = cells[2];
    }

    byId.set(id, entry);
  }

  const out: ParsedTaxon[] = [];
  for (const entry of byId.values()) {
    if (!entry.name) {
      issues.push({
        line: `taxid ${entry.id}`,
        reason: 'nodes.dmp row with no matching "scientific name" row from names.dmp',
      });
      continue;
    }
    if (!entry.rank) {
      issues.push({
        line: `${entry.id} — ${entry.name}`,
        reason: 'names.dmp row with no matching nodes.dmp row, so the rank is unknown',
      });
      continue;
    }
    out.push({
      ncbi_taxon_id: entry.id,
      name: entry.name,
      rank: entry.rank,
      // taxdump marks the root by making it its own parent; that would be a cycle.
      parent_ncbi_id: entry.parent === entry.id ? null : entry.parent ?? null,
      source: 'taxdump',
    });
  }
  return out;
}

/** CSV or TSV with a header row naming at least the id, name, and rank columns. */
function parseDelimited(text: string, issues: ParseIssue[]): ParsedTaxon[] {
  const lines = text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  if (lines.length === 0) return [];

  const header = splitDelimited(lines[0]).map(normalizeKey);
  const headerLooksReal = header.some(
    (h) => ID_KEYS.includes(h) || NAME_KEYS.includes(h) || RANK_KEYS.includes(h)
  );
  if (!headerLooksReal) {
    issues.push({
      line: lines[0].slice(0, 120),
      reason:
        'no recognisable header — name the columns taxid, name, rank (and optionally parent_taxid)',
    });
    return [];
  }

  const out: ParsedTaxon[] = [];
  for (let i = 1; i < lines.length; i++) {
    const cells = splitDelimited(lines[i]);
    const obj: Record<string, unknown> = {};
    header.forEach((key, idx) => {
      if (key) obj[key] = cells[idx] ?? '';
    });
    const rec = parseRecordObject(obj, `row ${i + 1}`, issues);
    if (rec) out.push(rec);
  }
  return out;
}

// ── E-utilities URL helpers (built, never fetched) ───────────────────────────

const EUTILS = 'https://eutils.ncbi.nlm.nih.gov/entrez/eutils';

/** URL that returns full records — including lineage — for these taxon IDs. */
export function buildEfetchUrl(ids: Array<number | string>): string {
  return `${EUTILS}/efetch.fcgi?db=taxonomy&id=${ids.join(',')}&retmode=xml`;
}

/** URL that returns summary records for these taxon IDs as JSON. */
export function buildEsummaryUrl(ids: Array<number | string>): string {
  return `${EUTILS}/esummary.fcgi?db=taxonomy&id=${ids.join(',')}&retmode=json`;
}

/** URL that resolves scientific names to taxon IDs. */
export function buildEsearchUrl(names: string[]): string {
  const term = names.map((n) => `${n.trim()}[Scientific Name]`).join(' OR ');
  return `${EUTILS}/esearch.fcgi?db=taxonomy&term=${encodeURIComponent(term)}&retmode=json`;
}

// ── Entry point ──────────────────────────────────────────────────────────────

/**
 * Parse whatever is in `input` into taxon records, reporting what it could not
 * use rather than dropping it. Never throws.
 */
export function parseNcbiInput(input: string): ParseResult {
  const format = detectFormat(input);
  const issues: ParseIssue[] = [];
  const notes: string[] = [];
  let records: ParsedTaxon[] = [];
  let lookupUrl: string | undefined;

  const text = input.trim();

  switch (format) {
    case 'empty':
      break;
    case 'record-json':
      records = parseRecordJson(text, issues);
      break;
    case 'esummary-json':
      records = parseEsummary(text, issues);
      break;
    case 'efetch-xml':
      records = parseEfetchXml(text, issues);
      break;
    case 'taxdump':
      records = parseTaxdump(text, issues);
      break;
    case 'delimited':
      records = parseDelimited(text, issues);
      break;
    case 'taxid-list': {
      const ids = text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
      notes.push(
        `${ids.length} taxon ID${ids.length === 1 ? '' : 's'} found, but a taxon ID alone has no ` +
          'name or rank. Open the link below, then paste the response back here.'
      );
      lookupUrl = buildEfetchUrl(ids);
      break;
    }
    case 'name-list': {
      const names = text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
      notes.push(
        `${names.length} scientific name${names.length === 1 ? '' : 's'} found, but NCBI records ` +
          'are keyed by taxon ID. Open the link below to look these up, then fetch and paste the ' +
          'full records.'
      );
      lookupUrl = buildEsearchUrl(names);
      break;
    }
    case 'unknown':
      issues.push({
        line: text.slice(0, 120),
        reason:
          'unrecognised format — paste E-utilities JSON or XML, taxdump rows, a CSV/TSV table ' +
          'with taxid/name/rank columns, or a plain list of taxon IDs',
      });
      break;
  }

  // De-duplicate, keeping the first occurrence. A lineage-expanded paste repeats
  // shared ancestors on every record; the backend collapses these too, but doing
  // it here means the preview shows the operator what will really be sent.
  const seen = new Set<number>();
  const deduped: ParsedTaxon[] = [];
  let duplicates = 0;
  for (const rec of records) {
    if (seen.has(rec.ncbi_taxon_id)) {
      duplicates++;
      continue;
    }
    seen.add(rec.ncbi_taxon_id);
    deduped.push(rec);
  }
  if (duplicates > 0) {
    notes.push(`${duplicates} duplicate record${duplicates === 1 ? '' : 's'} collapsed.`);
  }

  const belowGenus = deduped.filter((r) => !isBackboneRank(r.rank));
  if (belowGenus.length > 0) {
    notes.push(
      `${belowGenus.length} record${belowGenus.length === 1 ? '' : 's'} sit below genus ` +
        '(species, subspecies, "no rank"). The taxonomy backbone stores kingdom through genus ' +
        'only — species belong in the Species Registry — so these are excluded by default.'
    );
  }

  // Sort root-first so the preview reads top-down like the tree it builds.
  const rankOrder = new Map<string, number>(BACKBONE_RANKS.map((r, i) => [r as string, i]));
  const orderOf = (rank: string): number => {
    const key = rank.trim().toLowerCase();
    if (key === 'superkingdom') return 0;
    if (key === 'division') return 1;
    return rankOrder.get(key) ?? BACKBONE_RANKS.length;
  };
  deduped.sort((a, b) => orderOf(a.rank) - orderOf(b.rank));

  return {
    format,
    formatLabel: FORMAT_LABELS[format],
    records: deduped,
    issues,
    notes,
    lookupUrl,
  };
}

import { describe, it, expect } from 'vitest';
import {
  parseNcbiInput,
  detectFormat,
  isBackboneRank,
  buildEfetchUrl,
  buildEsummaryUrl,
  buildEsearchUrl,
} from './ncbiParse';

describe('detectFormat', () => {
  it('calls an empty string empty', () => {
    expect(detectFormat('')).toBe('empty');
    expect(detectFormat('   \n  ')).toBe('empty');
  });

  it('recognises a JSON array of records', () => {
    expect(detectFormat('[{"ncbi_taxon_id":5326,"name":"Pleurotus","rank":"genus"}]')).toBe('record-json');
  });

  it('distinguishes esummary from plain record JSON by its result envelope', () => {
    expect(detectFormat('{"header":{},"result":{"uids":["5326"]}}')).toBe('esummary-json');
    expect(detectFormat('{"ncbi_taxon_id":5326,"name":"Pleurotus","rank":"genus"}')).toBe('record-json');
  });

  it('recognises efetch XML', () => {
    expect(detectFormat('<?xml version="1.0"?><TaxaSet><Taxon/></TaxaSet>')).toBe('efetch-xml');
  });

  it('recognises taxdump rows by their tab-pipe-tab separator', () => {
    expect(detectFormat('5326\t|\t5325\t|\tgenus\t|')).toBe('taxdump');
  });

  it('recognises a delimited table by its header', () => {
    expect(detectFormat('taxid,name,rank\n5326,Pleurotus,genus')).toBe('delimited');
  });

  it('recognises a bare list of taxon IDs', () => {
    expect(detectFormat('5326\n5325\n4930')).toBe('taxid-list');
  });

  it('recognises a bare list of scientific names', () => {
    expect(detectFormat('Pleurotus ostreatus\nCitrus sinensis')).toBe('name-list');
  });
});

describe('parseNcbiInput — JSON records', () => {
  it('parses the documented array shape', () => {
    const r = parseNcbiInput('[{"ncbi_taxon_id":5326,"name":"Pleurotus","rank":"genus","parent_ncbi_id":5325}]');
    expect(r.records).toHaveLength(1);
    expect(r.records[0]).toMatchObject({
      ncbi_taxon_id: 5326,
      name: 'Pleurotus',
      rank: 'genus',
      parent_ncbi_id: 5325,
    });
  });

  it('accepts a single object as well as an array', () => {
    const r = parseNcbiInput('{"ncbi_taxon_id":5326,"name":"Pleurotus","rank":"genus"}');
    expect(r.records).toHaveLength(1);
    expect(r.records[0].parent_ncbi_id).toBeNull();
  });

  it('accepts NCBI field spellings as aliases', () => {
    const r = parseNcbiInput('[{"TaxId":"5326","ScientificName":"Pleurotus","Rank":"genus","ParentTaxId":"5325"}]');
    expect(r.records[0]).toMatchObject({ ncbi_taxon_id: 5326, name: 'Pleurotus', parent_ncbi_id: 5325 });
  });

  it('reports a record missing its rank instead of dropping it', () => {
    const r = parseNcbiInput('[{"ncbi_taxon_id":5326,"name":"Pleurotus"}]');
    expect(r.records).toHaveLength(0);
    expect(r.issues).toHaveLength(1);
    expect(r.issues[0].reason).toContain('rank');
  });

  it('reports malformed JSON as a parse error rather than guessing another format', () => {
    const r = parseNcbiInput('[{"ncbi_taxon_id":5326,');
    expect(r.records).toHaveLength(0);
    expect(r.issues[0].reason).toContain('JSON parse error');
  });

  it('keeps the good records when one record in the array is bad', () => {
    const r = parseNcbiInput(
      '[{"ncbi_taxon_id":5326,"name":"Pleurotus","rank":"genus"},{"name":"nameless"}]'
    );
    expect(r.records).toHaveLength(1);
    expect(r.issues).toHaveLength(1);
  });
});

describe('parseNcbiInput — esummary JSON', () => {
  const esummary = JSON.stringify({
    header: { type: 'esummary', version: '0.3' },
    result: {
      uids: ['5326'],
      '5326': {
        uid: '5326',
        scientificname: 'Pleurotus',
        rank: 'genus',
        parenttaxid: '5325',
      },
    },
  });

  it('reads records out of the result envelope', () => {
    const r = parseNcbiInput(esummary);
    expect(r.format).toBe('esummary-json');
    expect(r.records).toHaveLength(1);
    expect(r.records[0]).toMatchObject({ ncbi_taxon_id: 5326, name: 'Pleurotus', rank: 'genus', parent_ncbi_id: 5325 });
  });

  it('ignores the uids key when falling back to enumerating the result object', () => {
    const noUids = JSON.stringify({
      result: { '5326': { uid: '5326', scientificname: 'Pleurotus', rank: 'genus' } },
    });
    const r = parseNcbiInput(noUids);
    expect(r.records).toHaveLength(1);
  });

  it('flags a response with no result object', () => {
    const r = parseNcbiInput('{"result":null,"error":"boom"}');
    expect(r.records).toHaveLength(0);
    expect(r.issues.length).toBeGreaterThan(0);
  });
});

describe('parseNcbiInput — efetch XML', () => {
  const efetch = `<?xml version="1.0"?>
<TaxaSet>
  <Taxon>
    <TaxId>5322</TaxId>
    <ScientificName>Pleurotus ostreatus</ScientificName>
    <ParentTaxId>5326</ParentTaxId>
    <Rank>species</Rank>
    <LineageEx>
      <Taxon><TaxId>4751</TaxId><ScientificName>Fungi</ScientificName><Rank>kingdom</Rank></Taxon>
      <Taxon><TaxId>5204</TaxId><ScientificName>Basidiomycota</ScientificName><Rank>phylum</Rank></Taxon>
      <Taxon><TaxId>5326</TaxId><ScientificName>Pleurotus</ScientificName><Rank>genus</Rank></Taxon>
    </LineageEx>
  </Taxon>
</TaxaSet>`;

  it('expands the lineage into separate records', () => {
    const r = parseNcbiInput(efetch);
    expect(r.format).toBe('efetch-xml');
    const names = r.records.map((x) => x.name);
    expect(names).toContain('Fungi');
    expect(names).toContain('Basidiomycota');
    expect(names).toContain('Pleurotus');
    expect(names).toContain('Pleurotus ostreatus');
  });

  it('chains lineage parents from document order', () => {
    const r = parseNcbiInput(efetch);
    const byName = Object.fromEntries(r.records.map((x) => [x.name, x]));
    expect(byName['Fungi'].parent_ncbi_id).toBeNull();
    expect(byName['Basidiomycota'].parent_ncbi_id).toBe(4751);
    expect(byName['Pleurotus'].parent_ncbi_id).toBe(5204);
  });

  it('uses the record own ParentTaxId for the top-level taxon', () => {
    const r = parseNcbiInput(efetch);
    const species = r.records.find((x) => x.name === 'Pleurotus ostreatus');
    expect(species?.parent_ncbi_id).toBe(5326);
  });

  it('does not read a nested LineageEx TaxId as the outer taxon own ID', () => {
    // getElementsByTagName would return the kingdom TaxId first; only direct
    // children are the taxon's own fields.
    const r = parseNcbiInput(efetch);
    const species = r.records.find((x) => x.name === 'Pleurotus ostreatus');
    expect(species?.ncbi_taxon_id).toBe(5322);
  });

  it('notes that the species-rank record is below the backbone', () => {
    const r = parseNcbiInput(efetch);
    expect(r.notes.join(' ')).toContain('below genus');
  });

  it('reports malformed XML', () => {
    const r = parseNcbiInput('<TaxaSet><Taxon>');
    expect(r.records).toHaveLength(0);
    expect(r.issues.length).toBeGreaterThan(0);
  });
});

describe('parseNcbiInput — taxdump', () => {
  it('joins nodes.dmp and names.dmp rows on the taxon ID', () => {
    const input = [
      '5326\t|\t5204\t|\tgenus\t|\t\t|',
      '5326\t|\tPleurotus\t|\t\t|\tscientific name\t|',
    ].join('\n');
    const r = parseNcbiInput(input);
    expect(r.format).toBe('taxdump');
    expect(r.records).toHaveLength(1);
    expect(r.records[0]).toMatchObject({ ncbi_taxon_id: 5326, name: 'Pleurotus', rank: 'genus', parent_ncbi_id: 5204 });
  });

  it('ignores synonym and common-name rows so they cannot overwrite the scientific name', () => {
    const input = [
      '5326\t|\t5204\t|\tgenus\t|\t\t|',
      '5326\t|\tPleurotus\t|\t\t|\tscientific name\t|',
      '5326\t|\toyster mushrooms\t|\t\t|\tcommon name\t|',
    ].join('\n');
    const r = parseNcbiInput(input);
    expect(r.records[0].name).toBe('Pleurotus');
  });

  it('reports a nodes row with no matching name row', () => {
    const r = parseNcbiInput('5326\t|\t5204\t|\tgenus\t|\t\t|');
    expect(r.records).toHaveLength(0);
    expect(r.issues[0].reason).toContain('names.dmp');
  });

  it('drops the self-parent the root uses rather than creating a cycle', () => {
    const input = ['1\t|\t1\t|\tno rank\t|\t\t|', '1\t|\troot\t|\t\t|\tscientific name\t|'].join('\n');
    const r = parseNcbiInput(input);
    expect(r.records[0].parent_ncbi_id).toBeNull();
  });
});

describe('parseNcbiInput — delimited tables', () => {
  it('parses a TSV with a recognisable header', () => {
    const r = parseNcbiInput('taxid\tname\trank\tparent_taxid\n5326\tPleurotus\tgenus\t5204');
    expect(r.records).toHaveLength(1);
    expect(r.records[0]).toMatchObject({ ncbi_taxon_id: 5326, name: 'Pleurotus', parent_ncbi_id: 5204 });
  });

  it('parses a CSV and strips quotes', () => {
    const r = parseNcbiInput('taxid,name,rank\n5326,"Pleurotus",genus');
    expect(r.records[0].name).toBe('Pleurotus');
  });

  it('tolerates header spelling variations', () => {
    const r = parseNcbiInput('Tax ID,Scientific Name,Rank\n5326,Pleurotus,genus');
    expect(r.records).toHaveLength(1);
  });

  it('explains itself when the header is unrecognisable', () => {
    const r = parseNcbiInput('a,b,c\n1,2,3');
    expect(r.records).toHaveLength(0);
    expect(r.issues[0].reason).toContain('header');
  });
});

describe('parseNcbiInput — lists that need a round trip to NCBI', () => {
  it('turns a taxid list into an efetch link rather than an error', () => {
    const r = parseNcbiInput('5326\n5204');
    expect(r.records).toHaveLength(0);
    expect(r.issues).toHaveLength(0);
    expect(r.lookupUrl).toContain('efetch.fcgi');
    expect(r.lookupUrl).toContain('5326,5204');
    expect(r.notes[0]).toContain('taxon ID');
  });

  it('turns a name list into an esearch link', () => {
    const r = parseNcbiInput('Pleurotus ostreatus\nCitrus sinensis');
    expect(r.lookupUrl).toContain('esearch.fcgi');
    expect(r.notes[0]).toContain('scientific name');
  });
});

describe('parseNcbiInput — normalisation', () => {
  it('collapses duplicate taxon IDs and says so', () => {
    const r = parseNcbiInput(
      '[{"ncbi_taxon_id":5326,"name":"Pleurotus","rank":"genus"},{"ncbi_taxon_id":5326,"name":"Pleurotus","rank":"genus"}]'
    );
    expect(r.records).toHaveLength(1);
    expect(r.notes.join(' ')).toContain('duplicate');
  });

  it('sorts records root-first so the preview reads like the tree', () => {
    const r = parseNcbiInput(
      '[{"ncbi_taxon_id":3,"name":"G","rank":"genus"},' +
        '{"ncbi_taxon_id":1,"name":"K","rank":"kingdom"},' +
        '{"ncbi_taxon_id":2,"name":"F","rank":"family"}]'
    );
    expect(r.records.map((x) => x.rank)).toEqual(['kingdom', 'family', 'genus']);
  });

  it('never throws on hostile input', () => {
    for (const input of ['', '   ', '<<<', '{', '[]', ' ', 'null', '"a string"']) {
      expect(() => parseNcbiInput(input)).not.toThrow();
    }
  });
});

describe('isBackboneRank', () => {
  it('accepts the six backbone ranks and the NCBI spellings the backend folds', () => {
    for (const rank of ['kingdom', 'superkingdom', 'phylum', 'division', 'class', 'order', 'family', 'genus']) {
      expect(isBackboneRank(rank)).toBe(true);
    }
  });

  it('rejects ranks below genus', () => {
    for (const rank of ['species', 'subspecies', 'no rank', 'clade', 'strain']) {
      expect(isBackboneRank(rank)).toBe(false);
    }
  });

  it('is case- and whitespace-insensitive', () => {
    expect(isBackboneRank('  Genus ')).toBe(true);
  });
});

describe('E-utilities URL builders', () => {
  it('builds an efetch URL', () => {
    expect(buildEfetchUrl([1, 2])).toBe(
      'https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=taxonomy&id=1,2&retmode=xml'
    );
  });

  it('builds an esummary URL', () => {
    expect(buildEsummaryUrl([5326])).toContain('esummary.fcgi?db=taxonomy&id=5326');
  });

  it('encodes names in the esearch term', () => {
    const url = buildEsearchUrl(['Pleurotus ostreatus']);
    expect(url).toContain('esearch.fcgi');
    expect(url).toContain(encodeURIComponent('Pleurotus ostreatus[Scientific Name]'));
  });
});

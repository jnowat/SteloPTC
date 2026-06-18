<script lang="ts">
  let { records }: { records: any[] } = $props();
</script>

{#if records.length === 0}
  <div class="empty-state">No compliance records</div>
{:else}
  <table>
    <thead>
      <tr>
        <th title="Category of compliance record (e.g. phytosanitary test, import permit, CITES certificate)">Type</th>
        <th title="Regulatory agency or authority that issued or required this record">Agency</th>
        <th title="Specific test type performed or permit number associated with this record">Test / Permit</th>
        <th title="Outcome of the test (Positive = pathogen detected, Negative = clean, Pending = awaiting result)">Result</th>
        <th title="Current validity status of this compliance record (valid, pending, flagged, or expired)">Status</th>
        <th title="Date the test was performed or the compliance record was created">Date</th>
      </tr>
    </thead>
    <tbody>
      {#each records as cr}
        <tr>
          <td>{cr.record_type}</td>
          <td>{cr.agency || '—'}</td>
          <td>{cr.test_type || cr.permit_number || '—'}</td>
          <td>
            {#if cr.test_result === 'positive'}
              <span class="badge badge-red">Positive</span>
            {:else if cr.test_result === 'negative'}
              <span class="badge badge-green">Negative</span>
            {:else if cr.test_result === 'pending'}
              <span class="badge badge-yellow">Pending</span>
            {:else}
              {cr.test_result || '—'}
            {/if}
          </td>
          <td>
            <span class="badge"
              class:badge-green={cr.status === 'valid'}
              class:badge-red={cr.status === 'flagged' || cr.status === 'expired'}
              class:badge-yellow={cr.status === 'pending'}>
              {cr.status}
            </span>
          </td>
          <td>{cr.test_date || cr.created_at?.split(' ')[0] || '—'}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

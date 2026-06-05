<script lang="ts">
  let { admitted, rejected, warnings, policy } = $props<{
    admitted: number;
    rejected: number;
    warnings: number;
    policy: string;
  }>();
</script>

<div class="gauge-grid">
  <div class="stat-card admitted">
    <span class="stat-label">Admitted</span>
    <span class="stat-value">{admitted}</span>
  </div>
  <div class="stat-card rejected">
    <span class="stat-label">Rejected</span>
    <span class="stat-value">{rejected}</span>
  </div>
  <div class="stat-card warnings">
    <span class="stat-label">Advisory Warnings</span>
    <span class="stat-value">{warnings}</span>
  </div>
  <div class="stat-card policy">
    <span class="stat-label">Active Policy</span>
    <span class="stat-value">{policy}</span>
  </div>
</div>

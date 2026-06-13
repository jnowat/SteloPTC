<script lang="ts">
  import { changePassword } from '../api';
  import { mustChangePassword } from '../stores/auth';

  let newPassword = $state('');
  let confirmPassword = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';

    if (newPassword.length < 8) {
      error = 'Password must be at least 8 characters.';
      return;
    }
    if (newPassword !== confirmPassword) {
      error = 'Passwords do not match.';
      return;
    }

    loading = true;
    try {
      await changePassword(newPassword);
      mustChangePassword.set(false);
    } catch (err: any) {
      error = err.message || 'Failed to change password.';
    } finally {
      loading = false;
    }
  }
</script>

<div class="overlay">
  <div class="card">
    <div class="header">
      <h1>SteloPTC</h1>
      <h2>Set a New Password</h2>
      <p>For security, you must set a new password before continuing. The default password cannot be used.</p>
    </div>
    <form onsubmit={handleSubmit}>
      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
      <div class="form-group">
        <label for="new-password">New Password</label>
        <input id="new-password" type="password" bind:value={newPassword} placeholder="At least 8 characters" required minlength="8" autocomplete="new-password" />
      </div>
      <div class="form-group">
        <label for="confirm-password">Confirm Password</label>
        <input id="confirm-password" type="password" bind:value={confirmPassword} placeholder="Repeat new password" required autocomplete="new-password" />
      </div>
      <button type="submit" class="btn btn-primary submit-btn" disabled={loading}>
        {loading ? 'Saving...' : 'Set Password & Continue'}
      </button>
    </form>
  </div>
</div>

<style>
  .overlay {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    width: 100vw;
    background: linear-gradient(135deg, #0f172a 0%, #1a1100 50%, #0f4c2d 100%);
    position: fixed;
    top: 0;
    left: 0;
    z-index: 9999;
  }
  .card {
    background: white;
    border-radius: 12px;
    padding: 40px;
    width: 420px;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.4);
  }
  .header {
    text-align: center;
    margin-bottom: 28px;
  }
  .header h1 {
    font-size: 22px;
    font-weight: 800;
    color: #0f4c2d;
    letter-spacing: -0.5px;
  }
  .header h2 {
    font-size: 18px;
    font-weight: 700;
    color: #1e293b;
    margin-top: 8px;
  }
  .header p {
    color: #6b7280;
    font-size: 13px;
    margin-top: 8px;
    line-height: 1.5;
  }
  .submit-btn {
    width: 100%;
    padding: 12px;
    font-size: 15px;
    margin-top: 8px;
  }
  .error-msg {
    background: #fef2f2;
    color: #991b1b;
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 13px;
    margin-bottom: 16px;
    border: 1px solid #fecaca;
  }
</style>

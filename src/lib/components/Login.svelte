<script lang="ts">
  import { login } from '../api';
  import { setAuth } from '../stores/auth';

  let username = $state('');
  let password = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleLogin(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;
    try {
      const result = await login(username, password);
      setAuth(result.token, result.user);
    } catch (err: any) {
      error = err.message || 'Login failed';
    } finally {
      loading = false;
    }
  }
</script>

<div class="login-container">
  <div class="login-card">
    <div class="login-header">
      <h1>SteloPTC</h1>
      <p>Plant Tissue Culture Tracking System</p>
    </div>
    <form onsubmit={handleLogin}>
      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
      <div class="form-group">
        <label for="username">Username</label>
        <input id="username" type="text" bind:value={username} placeholder="Enter username" required />
      </div>
      <div class="form-group">
        <label for="password">Password</label>
        <input id="password" type="password" bind:value={password} placeholder="Enter password" required />
      </div>
      <button type="submit" class="btn btn-primary login-btn" disabled={loading}>
        {loading ? 'Signing in...' : 'Sign In'}
      </button>
      <p class="hint">Default: admin / admin</p>
    </form>
  </div>
</div>

<style>
  .login-container {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, #0f172a 0%, #1e3a5f 50%, #0f4c2d 100%);
  }
  .login-card {
    background: white;
    border-radius: 12px;
    padding: 40px;
    width: 400px;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.4);
  }
  .login-header {
    text-align: center;
    margin-bottom: 32px;
  }
  .login-header h1 {
    font-size: 28px;
    font-weight: 800;
    color: #0f4c2d;
    letter-spacing: -0.5px;
  }
  .login-header p {
    color: #6b7280;
    font-size: 14px;
    margin-top: 4px;
  }
  .login-btn {
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
  .hint {
    text-align: center;
    color: #9ca3af;
    font-size: 12px;
    margin-top: 16px;
  }
</style>

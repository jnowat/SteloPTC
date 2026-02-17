import App from './App.svelte';
import { mount } from 'svelte';

// Remove the loading screen once Svelte takes over
const loader = document.getElementById('app-loader');
if (loader) loader.remove();

const app = mount(App, {
  target: document.getElementById('app')!,
});

export default app;

import '@fontsource-variable/jetbrains-mono/wght.css';
import './styles.css';
import { mount } from 'svelte';
import App from './App.svelte';

const application = mount(App, {
  target: document.getElementById('app')!,
});

export default application;

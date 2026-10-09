import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import App from './App.svelte';

describe('App', () => {
  it('renders the stage zero application shell', () => {
    render(App);

    expect(screen.getByRole('heading', { name: 'Dockermon jest gotowy' })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Etap 0');
  });
});

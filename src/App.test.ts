import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import App from './App.svelte';
import { TestRuntimeClient } from './lib/runtime/TestRuntimeClient';

const renderApplication = (scenario: 'empty' | 'error' | 'loading' | 'ready' = 'ready') =>
  render(App, {
    runtimeClient: new TestRuntimeClient({ latency: 0, scenario }),
  });

describe('App', () => {
  it('renders projects and resource information from the test runtime', async () => {
    renderApplication();

    expect(await screen.findByRole('heading', { name: 'Local projects' })).toBeInTheDocument();
    expect(screen.getByTestId('project-storefront')).toHaveTextContent('Sklep lokalny');
    expect(screen.getByTestId('service-storefront-web')).toHaveTextContent('1.8%');
    expect(screen.getByTestId('service-storefront-web')).toHaveTextContent('284 MB');
    expect(screen.getByTestId('service-storefront-web')).toHaveTextContent('3000:3000');
  });

  it('keeps bulk selection scoped to its project action', async () => {
    renderApplication();
    const project = await screen.findByTestId('project-storefront');

    await fireEvent.click(within(project).getByRole('button', { name: 'Stop selected' }));

    await waitFor(() => {
      expect(screen.getByTestId('service-storefront-web')).toHaveTextContent('Stopped');
      expect(screen.getByTestId('service-storefront-postgres')).toHaveTextContent('Stopped');
      expect(screen.getByTestId('service-storefront-api')).toHaveTextContent('Healthy');
      expect(screen.getByTestId('service-api-local-api')).toHaveTextContent('Healthy');
    });
  });

  it('changes the theme and language without restarting', async () => {
    renderApplication();
    await screen.findByRole('heading', { name: 'Local projects' });

    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Dark' }));
    expect(document.documentElement.dataset.theme).toBe('dark');

    await fireEvent.change(screen.getByRole('combobox', { name: 'Language' }), {
      target: { value: 'pl' },
    });

    expect(await screen.findByRole('heading', { name: 'Preferencje' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Projekty' })).toBeInTheDocument();
  });

  it('opens, populates, and closes the log drawer from the keyboard', async () => {
    renderApplication();
    const project = await screen.findByTestId('project-api-local');

    const logTrigger = within(project).getByRole('button', { name: 'Show logs for api' });
    logTrigger.focus();
    expect(logTrigger).toHaveFocus();
    await fireEvent.click(logTrigger);

    const drawer = await screen.findByRole('complementary', { name: 'Logs · api' });
    expect(within(drawer).getByRole('button', { name: 'Close' })).toHaveFocus();
    const logOutput = await within(drawer).findByRole<HTMLTextAreaElement>('textbox', {
      name: 'Logs · api',
    });
    expect(logOutput.value).toContain('Server listening on :3000');

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('complementary', { name: 'Logs · api' })).not.toBeInTheDocument();
    expect(logTrigger).toHaveFocus();
  });

  it('shows deterministic empty and error states', async () => {
    const loadingRender = renderApplication('loading');
    expect(screen.getByLabelText('Reading local projects')).toBeInTheDocument();
    loadingRender.unmount();

    const emptyRender = renderApplication('empty');
    expect(
      await screen.findByRole('heading', { name: 'No local projects detected' }),
    ).toBeInTheDocument();
    emptyRender.unmount();

    renderApplication('error');
    expect(
      await screen.findByRole('heading', { name: 'Could not load Dockermon state' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });
});

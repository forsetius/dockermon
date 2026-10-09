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

  it('keeps lifecycle actions available in other projects while one project is busy', async () => {
    const runtimeClient = new TestRuntimeClient({ latency: 40 });
    render(App, { runtimeClient });
    const storefront = await screen.findByTestId('project-storefront');
    const localApi = screen.getByTestId('project-api-local');

    await fireEvent.click(within(storefront).getByRole('button', { name: 'Stop selected' }));

    expect(within(storefront).getByRole('button', { name: 'Stop selected' })).toBeDisabled();
    expect(within(localApi).getByRole('button', { name: 'Start selected' })).toBeEnabled();
  });

  it('blocks lifecycle actions and reports partial failures during a global stop', async () => {
    const runtimeClient = new TestRuntimeClient({
      globalStopFailures: ['storefront-web'],
      latency: 30,
    });
    render(App, { runtimeClient });
    const project = await screen.findByTestId('project-storefront');

    const stopPromise = runtimeClient.stopAll();

    expect(await screen.findByText('Preparing to stop all containers…')).toBeInTheDocument();
    expect(within(project).getByRole('button', { name: 'Stop selected' })).toBeDisabled();
    await stopPromise;

    expect(await screen.findByText('Stopped 7 of 8 containers.')).toBeInTheDocument();
    expect(screen.getByText('Could not stop web.')).toBeInTheDocument();
    expect(within(project).getByRole('button', { name: 'Stop selected' })).toBeEnabled();
  });

  it('disables actions that the connected runtime does not support', async () => {
    render(App, {
      runtimeClient: new TestRuntimeClient({
        capabilities: { lifecycleActions: false, logs: false },
        latency: 0,
      }),
    });
    const project = await screen.findByTestId('project-storefront');

    expect(within(project).getByRole('button', { name: 'Start selected' })).toBeDisabled();
    expect(within(project).getByRole('button', { name: 'Stop selected' })).toBeDisabled();
    expect(within(project).getByRole('button', { name: 'Stop web' })).toBeDisabled();
    expect(within(project).getByRole('button', { name: 'Restart web' })).toBeDisabled();
    expect(within(project).getByRole('button', { name: 'Show logs for web' })).toBeDisabled();
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

  it('imports a Compose project and enables an optional profile', async () => {
    renderApplication();
    await screen.findByRole('heading', { name: 'Local projects' });

    await fireEvent.click(screen.getByRole('button', { name: 'Add Compose files' }));

    const project = await screen.findByTestId('project-compose:zerniki');
    expect(within(project).getByTestId('service-compose:zerniki:postgres-test')).toHaveTextContent(
      'Profile disabled',
    );

    await fireEvent.click(within(project).getByRole('checkbox', { name: 'test' }));

    await waitFor(() => {
      expect(
        within(project).getByTestId('service-compose:zerniki:postgres-test'),
      ).toHaveTextContent('Not created');
    });
    expect(within(project).getByRole('checkbox', { name: 'Service: postgres-test' })).toBeChecked();

    expect(within(project).queryByRole('menuitem')).not.toBeInTheDocument();
    const menuTrigger = within(project).getByRole('button', { name: 'More actions for zerniki' });
    await fireEvent.click(menuTrigger);
    expect(within(project).getByRole('menuitem', { name: 'Remove from Dockermon' })).toHaveFocus();

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(within(project).queryByRole('menuitem')).not.toBeInTheDocument();
    expect(menuTrigger).toHaveFocus();

    await fireEvent.click(menuTrigger);
    await fireEvent.click(within(project).getByRole('menuitem', { name: 'Remove from Dockermon' }));

    await waitFor(() => {
      expect(screen.queryByTestId('project-compose:zerniki')).not.toBeInTheDocument();
    });
    expect(
      screen.getByText(
        'Removed zerniki from Dockermon. Containers and Compose files were not changed.',
      ),
    ).toBeInTheDocument();
  });

  it('opens, populates, and closes the log drawer from the keyboard', async () => {
    const runtimeClient = new TestRuntimeClient({ latency: 0, logInterval: 0 });
    render(App, { runtimeClient });
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
    expect(runtimeClient.activeLogSubscriptionCount()).toBe(0);
  });

  it('streams into a bounded buffer and clearing affects only the current view', async () => {
    const runtimeClient = new TestRuntimeClient({ latency: 0, logInterval: 0 });
    render(App, { runtimeClient });
    const project = await screen.findByTestId('project-api-local');

    await fireEvent.click(within(project).getByRole('button', { name: 'Show logs for api' }));
    const drawer = await screen.findByRole('complementary', { name: 'Logs · api' });
    const logOutput = await within(drawer).findByRole<HTMLTextAreaElement>('textbox', {
      name: 'Logs · api',
    });
    runtimeClient.emitServiceLogLines('api-local-api', ['live-before-clear']);
    await waitFor(() => expect(logOutput.value).toContain('live-before-clear'));

    await fireEvent.click(within(drawer).getByRole('button', { name: 'Clear view' }));
    expect(logOutput.value).toBe('');
    runtimeClient.emitServiceLogLines('api-local-api', ['live-after-clear']);
    await waitFor(() => expect(logOutput.value).toBe('live-after-clear'));

    runtimeClient.emitServiceLogLines(
      'api-local-api',
      Array.from({ length: 2005 }, (_, index) => `bounded-${index.toString().padStart(4, '0')}`),
    );
    await waitFor(() => {
      expect(logOutput.value).not.toContain('bounded-0000');
      expect(logOutput.value).toContain('bounded-2004');
    });
  });

  it('cancels the previous log stream when switching services', async () => {
    const runtimeClient = new TestRuntimeClient({ latency: 0, logInterval: 0 });
    render(App, { runtimeClient });
    const project = await screen.findByTestId('project-api-local');

    await fireEvent.click(within(project).getByRole('button', { name: 'Show logs for api' }));
    await screen.findByRole('complementary', { name: 'Logs · api' });
    await waitFor(() => expect(runtimeClient.activeLogSubscriptionCount()).toBe(1));

    await fireEvent.click(within(project).getByRole('button', { name: 'Show logs for web' }));
    const drawer = await screen.findByRole('complementary', { name: 'Logs · web' });
    const logOutput = await within(drawer).findByRole<HTMLTextAreaElement>('textbox', {
      name: 'Logs · web',
    });
    await waitFor(() => expect(runtimeClient.activeLogSubscriptionCount()).toBe(1));

    runtimeClient.emitServiceLogLines('api-local-api', ['old-service-line']);
    runtimeClient.emitServiceLogLines('api-local-web', ['current-service-line']);
    await waitFor(() => expect(logOutput.value).toContain('current-service-line'));
    expect(logOutput.value).not.toContain('old-service-line');
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

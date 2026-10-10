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

  it('collapses and expands the sidebar from the window header', async () => {
    renderApplication();
    await screen.findByRole('heading', { name: 'Local projects' });
    const appShell = document.querySelector<HTMLElement>('.app-shell');

    await fireEvent.click(screen.getByRole('button', { name: 'Collapse sidebar' }));
    expect(appShell).toHaveClass('sidebar-collapsed');

    await fireEvent.click(screen.getByRole('button', { name: 'Expand sidebar' }));
    expect(appShell).not.toHaveClass('sidebar-collapsed');
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
    const logOutput = await within(drawer).findByRole('log', {
      name: 'Logs · api',
    });
    expect(logOutput).toHaveTextContent('Server listening on :3000');

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('complementary', { name: 'Logs · api' })).not.toBeInTheDocument();
    expect(logTrigger).toHaveFocus();
    expect(runtimeClient.activeLogSubscriptionCount()).toBe(0);
  });

  it('resizes the right log drawer with pointer and keyboard controls', async () => {
    const originalWindowWidth = window.innerWidth;
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1920 });
    try {
      render(App, {
        runtimeClient: new TestRuntimeClient({ latency: 0, logInterval: 0 }),
      });
      const project = await screen.findByTestId('project-api-local');
      await fireEvent.click(within(project).getByRole('button', { name: 'Show logs for api' }));
      const resizeHandle = await screen.findByRole('slider', { name: 'Resize log drawer' });
      const appShell = document.querySelector<HTMLElement>('.app-shell');

      await waitFor(() => expect(resizeHandle).toHaveAttribute('aria-valuemax', '1100'));
      expect(appShell?.style.getPropertyValue('--log-drawer-width')).toBe('560px');
      await fireEvent.keyDown(resizeHandle, { key: 'ArrowLeft' });
      expect(appShell?.style.getPropertyValue('--log-drawer-width')).toBe('584px');
      await fireEvent.keyDown(resizeHandle, { key: 'Home' });
      expect(appShell?.style.getPropertyValue('--log-drawer-width')).toBe('360px');
      await fireEvent.keyDown(resizeHandle, { key: 'End' });
      expect(appShell?.style.getPropertyValue('--log-drawer-width')).toBe('1100px');

      await fireEvent.pointerDown(resizeHandle, {
        button: 0,
        clientX: 1120,
        pointerId: 1,
      });
      expect(appShell?.style.getPropertyValue('--log-drawer-width')).toBe('800px');
      await fireEvent.pointerMove(resizeHandle, { clientX: 920, pointerId: 1 });
      expect(appShell?.style.getPropertyValue('--log-drawer-width')).toBe('1000px');
      await fireEvent.pointerUp(resizeHandle, { clientX: 920, pointerId: 1 });
      await fireEvent.dblClick(resizeHandle);
      expect(appShell?.style.getPropertyValue('--log-drawer-width')).toBe('560px');
    } finally {
      Object.defineProperty(window, 'innerWidth', {
        configurable: true,
        value: originalWindowWidth,
      });
    }
  });

  it('docks logs at the bottom in a narrow workspace and resizes their height', async () => {
    const originalWindowWidth = window.innerWidth;
    const originalWindowHeight = window.innerHeight;
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1200 });
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: 900 });
    try {
      render(App, {
        runtimeClient: new TestRuntimeClient({ latency: 0, logInterval: 0 }),
      });
      const project = await screen.findByTestId('project-api-local');
      await fireEvent.click(within(project).getByRole('button', { name: 'Show logs for api' }));
      const drawer = await screen.findByRole('complementary', { name: 'Logs · api' });
      const resizeHandle = screen.getByRole('slider', { name: 'Resize log drawer' });
      const appShell = document.querySelector<HTMLElement>('.app-shell');

      expect(drawer).toHaveClass('drawer-bottom');
      expect(resizeHandle).toHaveAttribute('aria-orientation', 'vertical');
      await waitFor(() => expect(resizeHandle).toHaveAttribute('aria-valuemax', '558'));
      expect(appShell?.style.getPropertyValue('--log-drawer-height')).toBe('380px');

      await fireEvent.keyDown(resizeHandle, { key: 'ArrowUp' });
      expect(appShell?.style.getPropertyValue('--log-drawer-height')).toBe('404px');
      await fireEvent.keyDown(resizeHandle, { key: 'Home' });
      expect(appShell?.style.getPropertyValue('--log-drawer-height')).toBe('240px');
      await fireEvent.keyDown(resizeHandle, { key: 'End' });
      expect(appShell?.style.getPropertyValue('--log-drawer-height')).toBe('558px');

      await fireEvent.pointerDown(resizeHandle, {
        button: 0,
        clientY: 600,
        pointerId: 1,
      });
      expect(appShell?.style.getPropertyValue('--log-drawer-height')).toBe('300px');
      await fireEvent.pointerMove(resizeHandle, { clientY: 500, pointerId: 1 });
      expect(appShell?.style.getPropertyValue('--log-drawer-height')).toBe('400px');
      await fireEvent.pointerUp(resizeHandle, { clientY: 500, pointerId: 1 });
      await fireEvent.dblClick(resizeHandle);
      expect(appShell?.style.getPropertyValue('--log-drawer-height')).toBe('380px');

      Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1920 });
      await fireEvent(window, new Event('resize'));
      await waitFor(() => expect(drawer).toHaveClass('drawer-right'));
      expect(resizeHandle).toHaveAttribute('aria-orientation', 'horizontal');
    } finally {
      Object.defineProperty(window, 'innerWidth', {
        configurable: true,
        value: originalWindowWidth,
      });
      Object.defineProperty(window, 'innerHeight', {
        configurable: true,
        value: originalWindowHeight,
      });
    }
  });

  it('streams into a bounded buffer and clearing affects only the current view', async () => {
    const runtimeClient = new TestRuntimeClient({ latency: 0, logInterval: 0 });
    render(App, { runtimeClient });
    const project = await screen.findByTestId('project-api-local');

    await fireEvent.click(within(project).getByRole('button', { name: 'Show logs for api' }));
    const drawer = await screen.findByRole('complementary', { name: 'Logs · api' });
    const logOutput = await within(drawer).findByRole('log', {
      name: 'Logs · api',
    });
    runtimeClient.emitServiceLogLines('api-local-api', ['live-before-clear']);
    await waitFor(() => expect(logOutput).toHaveTextContent('live-before-clear'));

    await fireEvent.click(within(drawer).getByRole('button', { name: 'Clear' }));
    expect(logOutput).not.toHaveTextContent('live-before-clear');
    runtimeClient.emitServiceLogLines('api-local-api', ['live-after-clear']);
    await waitFor(() => expect(logOutput).toHaveTextContent('live-after-clear'));

    runtimeClient.emitServiceLogLines(
      'api-local-api',
      Array.from({ length: 2005 }, (_, index) => `bounded-${index.toString().padStart(4, '0')}`),
    );
    await waitFor(() => {
      expect(logOutput).not.toHaveTextContent('bounded-0000');
      expect(logOutput).toHaveTextContent('bounded-2004');
    });
  });

  it('renders HTTP and time log tokens with semantic classes', async () => {
    const runtimeClient = new TestRuntimeClient({ latency: 0, logInterval: 0 });
    render(App, { runtimeClient });
    const project = await screen.findByTestId('project-api-local');

    await fireEvent.click(within(project).getByRole('button', { name: 'Show logs for api' }));
    const drawer = await screen.findByRole('complementary', { name: 'Logs · api' });
    const logOutput = await within(drawer).findByRole('log', { name: 'Logs · api' });

    runtimeClient.emitServiceLogLines('api-local-api', [
      '2026-10-10T21:37:42.125Z GET /health 204',
    ]);

    await waitFor(() => expect(logOutput).toHaveTextContent('GET /health 204'));
    expect(logOutput.querySelector('.log-level-info')).toHaveTextContent('INFO');
    expect(logOutput.querySelector('.log-line:last-child .time')).toHaveTextContent(
      '21:37:42.125Z',
    );
    expect(logOutput.querySelector('.log-line:last-child .http-method-get')).toHaveTextContent(
      'GET',
    );
    expect(logOutput.querySelector('.log-line:last-child .http-status-success')).toHaveTextContent(
      '204',
    );
  });

  it('disables log coloring and retains the setting for the application session', async () => {
    const runtimeClient = new TestRuntimeClient({ latency: 0, logInterval: 0 });
    render(App, { runtimeClient });
    const project = await screen.findByTestId('project-api-local');
    const logTrigger = within(project).getByRole('button', { name: 'Show logs for api' });

    await fireEvent.click(logTrigger);
    let drawer = await screen.findByRole('complementary', { name: 'Logs · api' });
    let coloringToggle = within(drawer).getByRole('checkbox', { name: 'Coloring' });
    const logOutput = await within(drawer).findByRole('log', { name: 'Logs · api' });

    expect(coloringToggle).toBeChecked();
    runtimeClient.emitServiceLogLines('api-local-api', ['21:37:42 GET /health 204']);
    await waitFor(() => expect(logOutput.querySelector('.log-token')).toBeInTheDocument());

    await fireEvent.click(coloringToggle);
    expect(coloringToggle).not.toBeChecked();
    expect(logOutput).toHaveTextContent('21:37:42 GET /health 204');
    expect(logOutput.querySelector('.log-token')).not.toBeInTheDocument();

    await fireEvent.click(within(drawer).getByRole('button', { name: 'Close' }));
    await fireEvent.click(logTrigger);
    drawer = await screen.findByRole('complementary', { name: 'Logs · api' });
    coloringToggle = within(drawer).getByRole('checkbox', { name: 'Coloring' });
    expect(coloringToggle).not.toBeChecked();
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
    const logOutput = await within(drawer).findByRole('log', {
      name: 'Logs · web',
    });
    await waitFor(() => expect(runtimeClient.activeLogSubscriptionCount()).toBe(1));

    runtimeClient.emitServiceLogLines('api-local-api', ['old-service-line']);
    runtimeClient.emitServiceLogLines('api-local-web', ['current-service-line']);
    await waitFor(() => expect(logOutput).toHaveTextContent('current-service-line'));
    expect(logOutput).not.toHaveTextContent('old-service-line');
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

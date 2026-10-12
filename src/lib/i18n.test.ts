import { describe, expect, it } from 'vitest';
import { translate, translateActiveProjectCount } from './i18n';

describe('translate', () => {
  it('uses the concise Polish restart label', () => {
    expect(translate('pl', 'action.restart', { service: 'api' })).toBe('Zrestartuj api');
  });

  it('translates the resume action in both supported languages', () => {
    expect(translate('pl', 'action.resume', { service: 'worker' })).toBe('Wznów worker');
    expect(translate('en', 'action.resume', { service: 'worker' })).toBe('Resume worker');
  });
});

describe('translateActiveProjectCount', () => {
  it.each([
    [0, '0 aktywnych'],
    [1, '1 aktywny'],
    [2, '2 aktywne'],
    [5, '5 aktywnych'],
    [12, '12 aktywnych'],
    [22, '22 aktywne'],
  ])('uses the correct Polish form for %i', (count, expected) => {
    expect(translateActiveProjectCount('pl', count)).toBe(expected);
  });

  it('keeps the English count concise', () => {
    expect(translateActiveProjectCount('en', 5)).toBe('5 active');
  });
});

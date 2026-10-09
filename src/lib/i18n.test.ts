import { describe, expect, it } from 'vitest';
import { translateActiveProjectCount } from './i18n';

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

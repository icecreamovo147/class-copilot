import { describe, it, expect } from 'vitest';

describe('App initialization', () => {
  it('should have correct environment', () => {
    expect(typeof window).toBe('object');
    expect(typeof document).toBe('object');
  });

  it('should have basic math working', () => {
    expect(1 + 1).toBe(2);
  });
});

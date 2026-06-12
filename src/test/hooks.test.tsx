import { act, render, renderHook, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useKeyboardShortcut } from '@/hooks/useKeyboardShortcut';
import { useLocalStorageState } from '@/hooks/useLocalStorageState';

describe('useLocalStorageState', () => {
  it('reads the initial value from localStorage and persists updates', () => {
    window.localStorage.setItem('hook_test_key', JSON.stringify('已保存值'));

    const { result } = renderHook(() => useLocalStorageState('hook_test_key', '默认值'));

    expect(result.current[0]).toBe('已保存值');

    act(() => {
      result.current[1]('新值');
    });

    expect(window.localStorage.getItem('hook_test_key')).toBe(JSON.stringify('新值'));
  });
});

describe('useKeyboardShortcut', () => {
  function ShortcutHarness({ onTrigger }: { onTrigger: () => void }) {
    useKeyboardShortcut('k', onTrigger, { ctrlOrMeta: true });
    return <input aria-label="editor" />;
  }

  it('fires when the matching ctrl shortcut is pressed', () => {
    const onTrigger = vi.fn();
    render(<ShortcutHarness onTrigger={onTrigger} />);

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }));
    });

    expect(onTrigger).toHaveBeenCalledTimes(1);
  });

  it('ignores plain typing in editable fields without modifiers', () => {
    const onTrigger = vi.fn();

    function PlainShortcutHarness() {
      useKeyboardShortcut('n', onTrigger);
      return <input aria-label="editor" />;
    }

    render(<PlainShortcutHarness />);
    const input = screen.getByLabelText('editor');

    act(() => {
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true }));
    });

    expect(onTrigger).not.toHaveBeenCalled();
  });
});

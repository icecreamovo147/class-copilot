import { useEffect, useRef } from 'react';

interface ShortcutOptions {
  ctrlOrMeta?: boolean;
  shift?: boolean;
  alt?: boolean;
  enabled?: boolean;
  preventDefault?: boolean;
}

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false;
  const tagName = target.tagName.toLowerCase();
  return tagName === 'input' || tagName === 'textarea' || target.isContentEditable;
}

export function useKeyboardShortcut(
  key: string,
  handler: () => void,
  options: ShortcutOptions = {},
) {
  const {
    ctrlOrMeta = false,
    shift = false,
    alt = false,
    enabled = true,
    preventDefault = true,
  } = options;

  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!enabled) return;

    const listener = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target) && !(ctrlOrMeta || alt || shift)) {
        return;
      }
      if (event.key.toLowerCase() !== key.toLowerCase()) return;
      if (ctrlOrMeta && !(event.ctrlKey || event.metaKey)) return;
      if (!ctrlOrMeta && (event.ctrlKey || event.metaKey)) return;
      if (shift !== event.shiftKey) return;
      if (alt !== event.altKey) return;
      if (preventDefault) {
        event.preventDefault();
      }
      handlerRef.current();
    };

    window.addEventListener('keydown', listener);
    return () => window.removeEventListener('keydown', listener);
  }, [alt, ctrlOrMeta, enabled, key, preventDefault, shift]);
}

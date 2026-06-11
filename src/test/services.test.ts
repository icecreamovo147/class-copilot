import { describe, expect, it } from 'vitest';
import { getCommandErrorMessage, normalizeCommandArgs } from '@/services';

describe('Tauri command adapter', () => {
  it('converts only top-level command arguments to camelCase', () => {
    const records = [{ student_id: 1, status: '正常' }];

    expect(
      normalizeCommandArgs({
        cohort_id: 2,
        page_size: 20,
        optional_value: undefined,
        records,
      }),
    ).toEqual({
      cohortId: 2,
      pageSize: 20,
      records,
    });
  });

  it('preserves string errors returned by Tauri commands', () => {
    expect(getCommandErrorMessage('创建届次失败: 参数错误')).toBe(
      '创建届次失败: 参数错误',
    );
  });

  it('provides a fallback for unknown errors', () => {
    expect(getCommandErrorMessage(null)).toBe('操作失败，请稍后重试');
  });
});

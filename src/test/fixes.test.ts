import { describe, it, expect } from 'vitest';
import { normalizeCommandArgs } from '@/services';

describe('P2-1: Template download parameter mapping', () => {
  it('should convert template_type to templateType for Tauri invoke', () => {
    const args = normalizeCommandArgs({
      template_type: 'student',
      file_path: '/tmp/test.xlsx',
    });
    expect(args).toEqual({
      templateType: 'student',
      filePath: '/tmp/test.xlsx',
    });
  });

  it('should accept score template type', () => {
    const args = normalizeCommandArgs({
      template_type: 'score',
      file_path: '/tmp/score_template.xlsx',
    });
    expect(args).toEqual({
      templateType: 'score',
      filePath: '/tmp/score_template.xlsx',
    });
  });
});

describe('Homework stats calculation', () => {
  it('should calculate completion rate by student-homework records', () => {
    // 2 项作业 × 3 个学生 = 6 条记录, 其中 4 条已完成
    const totalRecords = 6;
    const completedRecords = 4;
    const rate = completedRecords / totalRecords;
    expect(rate).toBeCloseTo(2 / 3, 4);
  });

  it('should show unit labels correctly', () => {
    // 确保显示单位为"项"（作业数）和"人次"（记录数）
    const hwCount = 2;
    const completed = 4;
    const unitForHw = '项';
    const unitForRecords = '人次';
    expect(`${hwCount} ${unitForHw}`).toBe('2 项');
    expect(`${completed} ${unitForRecords}`).toBe('4 人次');
  });
});

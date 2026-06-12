import { invoke } from '@tauri-apps/api/core';
import { logger } from '@/utils/logger';
import type {
  Cohort,
  Student,
  Subject,
  ExamSubjectConfig,
  Homework,
  HomeworkRecord,
  Attendance,
  Exam,
  Score,
  BehaviorRecord,
  ClassFeeRecord,
  DashboardStats,
  SettingsOverview,
} from '@/types';

function toCamelCase(value: string): string {
  return value.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase());
}

export function normalizeCommandArgs(
  args?: Record<string, unknown>,
): Record<string, unknown> | undefined {
  if (!args) return undefined;

  return Object.fromEntries(
    Object.entries(args)
      .filter(([, value]) => value !== undefined)
      .map(([key, value]) => [toCamelCase(key), value]),
  );
}

export function getCommandErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === 'string' && error.trim()) {
    return error;
  }
  if (error && typeof error === 'object') {
    const message = Reflect.get(error, 'message');
    if (typeof message === 'string' && message.trim()) {
      return message;
    }
  }
  return '操作失败，请稍后重试';
}

// Tauri 默认将 Rust snake_case 命令参数暴露为 camelCase。
async function cmd<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const start = performance.now();
  try {
    const result = await invoke<T>(command, normalizeCommandArgs(args));
    const elapsed = (performance.now() - start).toFixed(1);
    logger.debug(`[IPC] ${command} ✓ ${elapsed}ms`);
    return result;
  } catch (error) {
    const elapsed = (performance.now() - start).toFixed(1);
    logger.error(`[IPC] ${command} ✗ ${elapsed}ms`, error);
    throw new Error(getCommandErrorMessage(error));
  }
}

// ==================== Cohort Service ====================
export const cohortService = {
  list: (params?: { search?: string; status?: string }) =>
    cmd<Cohort[]>('get_cohorts', params as Record<string, unknown>),

  getById: (id: number) => cmd<Cohort>('get_cohort', { id }),

  create: (data: Partial<Cohort>) => cmd<Cohort>('create_cohort', data as Record<string, unknown>),

  update: (id: number, data: Partial<Cohort>) =>
    cmd<Cohort>('update_cohort', { id, ...data }),

  archive: (id: number) => cmd<void>('archive_cohort', { id }),

  unarchive: (id: number) => cmd<void>('unarchive_cohort', { id }),

  setCurrent: (id: number) => cmd<void>('set_current_cohort', { id }),

  getCurrent: () => cmd<Cohort | null>('get_current_cohort'),
};

// ==================== Student Service ====================
export const studentService = {
  list: (cohortId: number, params?: { search?: string; gender?: string; group_name?: string; status?: string; is_focus?: boolean; page?: number; page_size?: number }) =>
    cmd<{ data: Student[]; total: number; page: number; page_size: number }>('get_students', { cohort_id: cohortId, ...params }),

  getAll: (cohortId: number) => cmd<Student[]>('get_all_students', { cohort_id: cohortId }),

  getById: (id: number) => cmd<Student>('get_student', { id }),

  create: (data: Partial<Student>) => cmd<Student>('create_student', data),

  update: (id: number, data: Partial<Student>) => cmd<Student>('update_student', { id, ...data }),

  delete: (id: number) => cmd<void>('delete_student', { id }),

  previewExcel: (cohortId: number, filePath: string) =>
    cmd<{ total_rows: number; valid_rows: number; error_rows: number; rows: Array<Record<string, unknown>>; errors: string[] }>(
      'preview_students_excel', { cohort_id: cohortId, file_path: filePath }
    ),

  importExcel: (cohortId: number, filePath: string) =>
    cmd<{ success: number; errors: string[] }>('import_students_excel', { cohort_id: cohortId, file_path: filePath }),

  exportExcel: (cohortId: number, filePath: string) =>
    cmd<void>('export_students_excel', { cohort_id: cohortId, file_path: filePath }),
};

// ==================== Subject Service ====================
export const subjectService = {
  list: (params?: { active_only?: boolean }) => cmd<Subject[]>('get_subjects', params),
  create: (data: Partial<Subject>) => cmd<Subject>('create_subject', data),
  update: (id: number, data: Partial<Subject>) => cmd<Subject>('update_subject', { id, ...data }),
  delete: (id: number) => cmd<void>('delete_subject', { id }),
};

// ==================== Homework Service ====================
export const homeworkService = {
  list: (cohortId: number, params?: { search?: string; subject_id?: number; publish_date?: string; incomplete_only?: boolean; page?: number; page_size?: number }) =>
    cmd<{ data: Homework[]; total: number; page: number; page_size: number }>('get_homeworks', { cohort_id: cohortId, ...params }),

  getById: (id: number) => cmd<Homework>('get_homework', { id }),

  create: (data: Partial<Homework> & { assigned_student_ids?: number[] }) => cmd<Homework>('create_homework', data),

  update: (id: number, data: Partial<Homework>) => cmd<Homework>('update_homework', { id, ...data }),

  delete: (id: number) => cmd<void>('delete_homework', { id }),

  getRecords: (homeworkId: number) => cmd<HomeworkRecord[]>('get_homework_records', { homework_id: homeworkId }),

  getStudentRecords: (studentId: number) => cmd<HomeworkRecord[]>('get_student_homework_records', { student_id: studentId }),

  updateRecord: (recordId: number, status: string, remark?: string) =>
    cmd<void>('update_homework_record', { id: recordId, status, remark }),

  batchUpdateRecords: (homeworkId: number, studentIds: number[], status: string) =>
    cmd<void>('batch_update_homework_records', { homework_id: homeworkId, student_ids: studentIds, status }),

  openAttachment: (id: number) => cmd<void>('open_homework_attachment', { id }),

  exportIncomplete: (homeworkId: number, filePath: string) =>
    cmd<void>('export_incomplete_homework', { homework_id: homeworkId, file_path: filePath }),
};

// ==================== Attendance Service ====================
export const attendanceService = {
  getByDate: (cohortId: number, date: string) =>
    cmd<Attendance[]>('get_attendance_by_date', { cohort_id: cohortId, date }),

  saveAll: (cohortId: number, date: string, records: Array<{ student_id: number; status: string; reason?: string; remark?: string; leave_type?: string; leave_start_date?: string; leave_end_date?: string }>) =>
    cmd<void>('save_attendance', { cohort_id: cohortId, date, records }),

  setAllNormal: (cohortId: number, date: string) =>
    cmd<void>('set_all_attendance_normal', { cohort_id: cohortId, date }),

  query: (cohortId: number, params?: { start_date?: string; end_date?: string; student_id?: number; status?: string; page?: number; page_size?: number }) =>
    cmd<{ data: Attendance[]; total: number; page: number; page_size: number }>('query_attendance', { cohort_id: cohortId, ...params }),

  statistics: (cohortId: number, startDate: string, endDate: string) =>
    cmd<Array<{ student_id: number; student_name: string; student_no: string; total: number; normal: number; late: number; early: number; leave: number; absent: number; attendance_rate: number }>>(
      'attendance_statistics', { cohort_id: cohortId, start_date: startDate, end_date: endDate }
    ),

  exportExcel: (cohortId: number, filePath: string, params?: { start_date?: string; end_date?: string }) =>
    cmd<void>('export_attendance_excel', { cohort_id: cohortId, file_path: filePath, ...params }),
};

// ==================== Exam & Score Service ====================
export const examService = {
  list: (cohortId: number) => cmd<Exam[]>('get_exams', { cohort_id: cohortId }),
  create: (data: Partial<Exam>) => cmd<Exam>('create_exam', data),
  update: (id: number, data: Partial<Exam>) => cmd<Exam>('update_exam', { id, ...data }),
  delete: (id: number) => cmd<void>('delete_exam', { id }),
  getSubjectConfigs: (examId: number) => cmd<ExamSubjectConfig[]>('get_exam_subject_configs', { exam_id: examId }),
  saveSubjectConfigs: (examId: number, configs: ExamSubjectConfig[]) =>
    cmd<void>('save_exam_subject_configs', { exam_id: examId, configs }),
};

export const scoreService = {
  getByExam: (examId: number, subjectId: number) =>
    cmd<Score[]>('get_scores_by_exam', { exam_id: examId, subject_id: subjectId }),

  save: (examId: number, subjectId: number, scores: Array<{ student_id: number; score_value: number | null; remark?: string }>) =>
    cmd<void>('save_scores', { exam_id: examId, subject_id: subjectId, scores }),

  previewExcel: (examId: number, subjectId: number, filePath: string) =>
    cmd<{
      total_rows: number;
      valid_rows: number;
      error_rows: number;
      rows: Array<Record<string, unknown>>;
      errors: string[];
      warnings: string[];
    }>('preview_scores_excel', { exam_id: examId, subject_id: subjectId, file_path: filePath }),

  importExcel: (examId: number, subjectId: number, filePath: string) =>
    cmd<{ success: number; errors: string[]; warnings: string[] }>('import_scores_excel', { exam_id: examId, subject_id: subjectId, file_path: filePath }),

  statistics: (examId: number, subjectId: number) =>
    cmd<{ avg_score: number; max_score: number; min_score: number; pass_rate: number; excellent_rate: number; full_score: number; pass_score: number; excellent_score: number }>(
      'score_statistics', { exam_id: examId, subject_id: subjectId }
    ),

  rankings: (examId: number) =>
    cmd<Array<{ student_id: number; student_name: string; student_no: string; total_score: number; rank_no: number }>>(
      'score_rankings', { exam_id: examId }
    ),

  exportExcel: (examId: number, subjectId: number, filePath: string) =>
    cmd<void>('export_scores_excel', { exam_id: examId, subject_id: subjectId, file_path: filePath }),
};

// ==================== Affairs Service ====================
export const noticeService = {
  list: (cohortId: number, params?: { search?: string; page?: number; page_size?: number }) =>
    cmd<{ data: any[]; total: number; page: number; page_size: number }>('get_notices', { cohort_id: cohortId, ...params }),
  create: (data: any) => cmd<any>('create_notice', data),
  update: (id: number, data: any) => cmd<any>('update_notice', { id, ...data }),
  delete: (id: number) => cmd<void>('delete_notice', { id }),
  exportExcel: (cohortId: number, filePath: string, params?: { search?: string }) =>
    cmd<void>('export_notices_excel', { cohort_id: cohortId, file_path: filePath, ...params }),
};

export const dutyService = {
  list: (cohortId: number, params?: { search?: string; status?: string; page?: number; page_size?: number }) =>
    cmd<{ data: any[]; total: number; page: number; page_size: number }>('get_duties', { cohort_id: cohortId, ...params }),
  create: (data: any) => cmd<any>('create_duty', data),
  update: (id: number, data: any) => cmd<any>('update_duty', { id, ...data }),
  delete: (id: number) => cmd<void>('delete_duty', { id }),
  exportExcel: (cohortId: number, filePath: string, params?: { status?: string }) =>
    cmd<void>('export_duties_excel', { cohort_id: cohortId, file_path: filePath, ...params }),
};

export const behaviorService = {
  list: (cohortId: number, params?: { student_id?: number; type?: string; page?: number; page_size?: number }) =>
    cmd<{ data: any[]; total: number; page: number; page_size: number }>('get_behavior_records', {
      cohort_id: cohortId,
      student_id: params?.student_id,
      record_type: params?.type,
      page: params?.page,
      page_size: params?.page_size,
    }),
  create: (data: any) => cmd<any>('create_behavior_record', { ...data, record_type: data.type }),
  delete: (id: number) => cmd<void>('delete_behavior_record', { id }),
};

export const classFeeService = {
  list: (cohortId: number, params?: { fee_type?: string; student_id?: number; payment_status?: string; page?: number; page_size?: number }) =>
    cmd<{
      data: ClassFeeRecord[];
      total: number;
      page: number;
      page_size: number;
      summary: { income_total: number; expense_total: number; balance: number; outstanding_total: number };
    }>('get_class_fee_records', { cohort_id: cohortId, ...params }),
  create: (data: Partial<ClassFeeRecord>) => cmd<ClassFeeRecord>('create_class_fee_record', data),
  update: (id: number, data: Partial<ClassFeeRecord>) => cmd<ClassFeeRecord>('update_class_fee_record', { id, ...data }),
  delete: (id: number) => cmd<void>('delete_class_fee_record', { id }),
  exportExcel: (cohortId: number, filePath: string, params?: { fee_type?: string; student_id?: number; payment_status?: string }) =>
    cmd<void>('export_class_fee_excel', { cohort_id: cohortId, file_path: filePath, ...params }),
};

// ==================== Statistics Service ====================
export const statisticsService = {
  dashboard: (cohortId: number) => cmd<DashboardStats>('get_dashboard_stats', { cohort_id: cohortId }),

  homeworkStats: (cohortId: number) =>
    cmd<{ total: number; avg_rate: number; total_incomplete: number; consecutive_incomplete: Array<{ student_id: number; student_name: string; student_no: string; count: number }> }>(
      'homework_statistics', { cohort_id: cohortId }
    ),

  homeworkTrend: (cohortId: number) =>
    cmd<Array<{ homework_id: number; title: string; publish_date: string; total_count: number; completed_count: number; incomplete_count: number; completion_rate: number }>>(
      'homework_trend_statistics', { cohort_id: cohortId }
    ),

  attendanceStats: (cohortId: number, startDate: string, endDate: string) =>
    cmd<{ total_days: number; records: Array<{ student_id: number; student_name: string; student_no: string; total: number; normal: number; late: number; early: number; leave: number; absent: number; rate: number }> }>(
      'attendance_statistics_cohort', { cohort_id: cohortId, start_date: startDate, end_date: endDate }
    ),

  attendanceTrend: (cohortId: number, startDate: string, endDate: string) =>
    cmd<Array<{ attendance_date: string; total_count: number; normal_count: number; late_count: number; early_count: number; leave_count: number; absent_count: number; normal_rate: number }>>(
      'attendance_trend_statistics', { cohort_id: cohortId, start_date: startDate, end_date: endDate }
    ),

  scoreStats: (cohortId: number) =>
    cmd<{ exams_count: number; subjects_count: number; records: Array<{ exam_name: string; subject_name: string; avg_score: number; max_score: number; min_score: number }> }>(
      'score_statistics_cohort', { cohort_id: cohortId }
    ),

  scoreTrend: (cohortId: number) =>
    cmd<Array<{ exam_id: number; exam_name: string; exam_point: string; subject_name: string; avg_score: number }>>(
      'score_trend_statistics', { cohort_id: cohortId }
    ),

  crossCohortComparison: (cohortIds: number[]) =>
    cmd<Array<{
      cohort_id: number;
      cohort_name: string;
      class_name: string;
      status: string;
      student_count: number;
      homework_completion_rate: number;
      attendance_rate: number;
      avg_score: number;
      missing_score_data: boolean;
      behavior_count: number;
      behavior_score_total: number;
    }>>('cross_cohort_comparison', { cohort_ids: cohortIds }),

  exportCrossCohortComparison: (cohortIds: number[], filePath: string) =>
    cmd<void>('export_cross_cohort_comparison', { cohort_ids: cohortIds, file_path: filePath }),

  exportCrossCohortComparisonPdf: (cohortIds: number[], filePath: string) =>
    cmd<void>('export_cross_cohort_comparison_pdf', { cohort_ids: cohortIds, file_path: filePath }),

  exportCohortStatisticsExcel: (cohortId: number, filePath: string) =>
    cmd<void>('export_cohort_statistics_excel', { cohort_id: cohortId, file_path: filePath }),

  exportCohortStatisticsPdf: (cohortId: number, filePath: string) =>
    cmd<void>('export_cohort_statistics_pdf', { cohort_id: cohortId, file_path: filePath }),

  studentProfile: (studentId: number) =>
    cmd<{
      student: Student;
      homework: { total: number; completed: number; rate: number; consecutive_incomplete: number };
      attendance: { total: number; normal: number; abnormal: number; rate: number };
      scores: Array<{ exam_name: string; exam_point: string; subject_name: string; score_value: number | null }>;
      score_trend: Array<{ exam_name: string; exam_point: string; subject_name: string; score_value: number | null }>;
      behaviors: BehaviorRecord[];
      focus_reasons: string[];
      overall_evaluation: string;
    }>('get_student_profile', { student_id: studentId }),

  exportStudentGrowthArchive: (studentId: number, filePath: string) =>
    cmd<void>('export_student_growth_archive', { student_id: studentId, file_path: filePath }),

  exportStudentGrowthArchivePdf: (studentId: number, filePath: string) =>
    cmd<void>('export_student_growth_archive_pdf', { student_id: studentId, file_path: filePath }),
};

// ==================== Backup Service ====================
export const backupService = {
  create: (filePath: string) => cmd<void>('create_backup', { file_path: filePath }),
  restore: (filePath: string) => cmd<void>('restore_backup', { file_path: filePath }),
  exportCohort: (cohortId: number, filePath: string) =>
    cmd<void>('export_cohort', { cohort_id: cohortId, file_path: filePath }),
};

// ==================== System Config Service ====================
export const configService = {
  get: (key: string) => cmd<string | null>('get_config', { key }),
  set: (key: string, value: string) => cmd<void>('set_config', { key, value }),
  getOverview: () => cmd<SettingsOverview>('get_settings_overview'),
  savePreferences: (data: {
    school_name?: string;
    head_teacher?: string;
    default_semester?: string;
    default_backup_dir?: string;
    reminder_threshold?: number;
    export_preference?: 'xlsx' | 'pdf' | 'both';
  }) => cmd<void>('save_settings_preferences', data as Record<string, unknown>),
  recentBackups: () => cmd<SettingsOverview['recent_backups']>('get_recent_backups'),
  downloadTemplate: (type: string, filePath: string) =>
    cmd<void>('download_template', { template_type: type, file_path: filePath }),
};

// ==================== Export Service ====================
export const exportService = {
  exportHomework: (homeworkId: number, filePath: string) =>
    cmd<void>('export_homework', { homework_id: homeworkId, file_path: filePath }),
};

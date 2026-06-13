// 通用类型定义

export interface Cohort {
  id: number;
  cohort_name: string;
  class_name: string;
  grade_name: string | null;
  school_name: string | null;
  head_teacher: string | null;
  admission_year: number | null;
  graduation_year: number | null;
  semester: string | null;
  status: CohortStatus;
  is_current: boolean;
  archive_time: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
}

export type CohortStatus = '使用中' | '已归档';

export interface Student {
  id: number;
  cohort_id: number;
  name: string;
  student_no: string;
  gender: string | null;
  phone: string | null;
  parent_name: string | null;
  parent_phone: string | null;
  address: string | null;
  group_name: string | null;
  status: string;
  is_focus: boolean;
  remark: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface Subject {
  id: number;
  name: string;
  sort_order: number;
  is_active: boolean;
  remark: string | null;
  created_at: string;
  updated_at: string;
}

export interface ExamSubjectConfig {
  id?: number;
  exam_id?: number;
  subject_id: number;
  subject_name?: string;
  full_score: number;
  pass_score: number;
  excellent_score: number;
  sort_order: number;
  is_active?: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface Homework {
  id: number;
  cohort_id: number;
  title: string;
  subject_id: number | null;
  subject_name: string | null;
  description: string | null;
  attachment_name?: string | null;
  attachment_path?: string | null;
  publish_date: string;
  deadline: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  completed_count?: number;
  total_count?: number;
  completion_rate?: number;
  incomplete_count?: number;
  assigned_student_ids?: number[];
}

export type HomeworkStatus = '未登记' | '已完成' | '未完成' | '迟交' | '补交' | '质量较差';

export interface HomeworkRecord {
  id: number;
  homework_id: number;
  student_id: number;
  status: HomeworkStatus;
  submit_time: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
  student_name?: string;
  student_no?: string;
  group_name?: string;
}

export type AttendanceStatus = '正常' | '迟到' | '早退' | '请假' | '旷课';

export interface Attendance {
  id: number;
  cohort_id: number;
  student_id: number;
  attendance_date: string;
  status: AttendanceStatus;
  leave_type?: string | null;
  leave_start_date?: string | null;
  leave_end_date?: string | null;
  reason: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
  student_name?: string;
  student_no?: string;
  group_name?: string;
}

export interface Exam {
  id: number;
  cohort_id: number;
  name: string;
  exam_type: string | null;
  exam_date: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface Score {
  id: number;
  exam_id: number;
  subject_id: number;
  student_id: number;
  score_value: number | null;
  rank_no: number | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
  student_name?: string;
  student_no?: string;
  subject_name?: string;
  exam_name?: string;
}

export interface Notice {
  id: number;
  cohort_id: number;
  title: string;
  content: string | null;
  publish_date: string;
  remark: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface Duty {
  id: number;
  cohort_id: number;
  duty_date: string;
  student_id: number | null;
  group_name: string | null;
  duty_content: string | null;
  status: string;
  remark: string | null;
  created_at: string;
  updated_at: string;
  student_name?: string;
}

export interface BehaviorRecord {
  id: number;
  cohort_id: number;
  student_id: number;
  type: string;
  title: string;
  score: number;
  description: string | null;
  record_date: string;
  created_at: string;
  updated_at: string;
  student_name?: string;
  student_no?: string;
}

export interface ClassFeeRecord {
  id: number;
  cohort_id: number;
  fee_date: string;
  fee_type: string;
  category: string | null;
  title: string;
  amount: number;
  student_id: number | null;
  payment_status: string | null;
  voucher_path: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  student_name?: string;
  student_no?: string;
}

export interface SystemConfig {
  id: number;
  config_key: string;
  config_value: string | null;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface PaginationParams {
  page: number;
  page_size: number;
}

export interface PaginatedResult<T> {
  data: T[];
  total: number;
  page: number;
  page_size: number;
}

export interface ApiResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface DashboardStats {
  cohort_name: string;
  class_name: string;
  status: string;
  total_students: number;
  male_count: number;
  female_count: number;
  today_homework_count: number;
  today_homework_total_records: number;
  today_homework_completed: number;
  today_homework_rate: number;
  today_attendance_normal: number;
  today_attendance_late: number;
  today_attendance_leave: number;
  today_attendance_absent: number;
  today_attendance_early: number;
  pending_homework: number;
  pending_attendance: boolean;
  focus_students: Array<{
    id: number;
    name: string;
    student_no: string;
    reason: string;
  }>;
}

export interface SettingsOverview {
  school_name: string | null;
  head_teacher: string | null;
  default_semester: string | null;
  default_backup_dir: string;
  app_version: string;
  database_version: number;
  data_dir: string;
  database_path: string;
  recent_backups: Array<{
    file_name: string;
    file_path: string;
    size_bytes: number;
    modified_at: string;
  }>;
}

// 业务常量
export const HOMEWORK_STATUSES: HomeworkStatus[] = ['未登记', '已完成', '未完成', '迟交', '补交', '质量较差'];
export const ATTENDANCE_STATUSES: AttendanceStatus[] = ['正常', '迟到', '早退', '请假', '旷课'];
export const STUDENT_STATUSES = ['正常', '休学', '退学', '转学'];
export const BEHAVIOR_TYPES = ['表扬', '违纪', '加分', '减分'];
export const GENDERS = ['男', '女'];

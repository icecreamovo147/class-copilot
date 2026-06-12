import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Card, Table, Button, DatePicker, Select, Tag, message, Typography,
  Space, Empty, Modal, Input, Tabs, Badge, Dropdown,
} from 'antd';
import type { InputRef } from 'antd';
import {
  CheckCircleOutlined, DownloadOutlined,
  SmileOutlined, ClockCircleOutlined, MinusCircleOutlined,
  HomeOutlined, StopOutlined, DownOutlined,
} from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useSearchParams } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { attendanceService, studentService } from '@/services';
import type { Attendance, AttendanceStatus } from '@/types';
import { ATTENDANCE_STATUSES } from '@/types';
import { useLocalStorageState } from '@/hooks/useLocalStorageState';
import { useIsDark } from '@/hooks/useTheme';
import dayjs from 'dayjs';

const { Title, Text } = Typography;
const { RangePicker } = DatePicker;
const LEAVE_TYPES = ['病假', '事假', '活动假', '其他'];
const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

const STATUS_CONFIG: Record<string, { color: string; bg: string; icon: React.ReactNode }> = {
  '正常': { color: '#52c41a', bg: '#f6ffed', icon: <SmileOutlined /> },
  '迟到': { color: '#fa8c16', bg: '#fff7e6', icon: <ClockCircleOutlined /> },
  '早退': { color: '#faad14', bg: '#fffbe6', icon: <MinusCircleOutlined /> },
  '请假': { color: '#1677ff', bg: '#e6f4ff', icon: <HomeOutlined /> },
  '旷课': { color: '#ff4d4f', bg: '#fff1f0', icon: <StopOutlined /> },
};

const STATUS_LABEL_MAP: Record<string, string> = {
  '正常': '正常',
  '迟到': '迟到',
  '早退': '早退',
  '请假': '请假',
  '旷课': '旷课',
};

// Tag 颜色映射 — 直接匹配 Ant Design Tag color 属性
const attendanceStatusConfig: Record<string, { label: string; color: string }> = {
  '正常': { label: '正常', color: 'success' },
  '迟到': { label: '迟到', color: 'orange' },
  '早退': { label: '早退', color: 'warning' },
  '请假': { label: '请假', color: 'processing' },
  '旷课': { label: '旷课', color: 'error' },
};

// ─── Stat Card ────────────────────────────────────────
function StatCard({ label, count, color, bg, icon, isDark: dark }: {
  label: string; count: number; color: string; bg: string; icon: React.ReactNode; isDark: boolean;
}) {
  return (
    <div style={{
      flex: '1 1 0',
      minWidth: 100,
      background: bg,
      borderRadius: 8,
      padding: '12px 16px',
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      border: `1px solid ${color}18`,
    }}>
      <div style={{
        width: 36, height: 36, borderRadius: 8,
        background: `${color}1a`, display: 'flex',
        alignItems: 'center', justifyContent: 'center',
        fontSize: 18, color,
      }}>
        {icon}
      </div>
      <div>
        <div style={{ fontSize: 12, color: dark ? '#8c8c8c' : '#8c8c8c', lineHeight: '18px' }}>{label}</div>
        <div style={{ fontSize: 24, fontWeight: 700, color, lineHeight: '30px' }}>{count}</div>
      </div>
    </div>
  );
}

// ─── Inline-Editable Text Cell ──────────────────────
function EditableTextCell({
  value, placeholder, disabled, onSave, isDark: dark,
}: {
  value: string | null;
  placeholder: string;
  disabled: boolean;
  onSave: (val: string) => void;
  isDark: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value || '');
  const inputRef = useRef<InputRef>(null);
  const textDisabled = dark ? '#8c8c8c' : '#595959';
  const textPlaceholder = dark ? '#595959' : '#bfbfbf';

  useEffect(() => {
    if (editing) {
      // focus after antd Input mounts
      setTimeout(() => inputRef.current?.focus?.(), 0);
    }
  }, [editing]);

  const commit = useCallback(() => {
    const trimmed = draft.trim();
    if (trimmed !== (value || '')) {
      onSave(trimmed);
    }
    setEditing(false);
  }, [draft, value, onSave]);

  if (editing) {
    return (
      <Input
        ref={inputRef}
        size="small"
        value={draft}
        placeholder={placeholder}
        style={{ width: '100%' }}
        onChange={(e) => setDraft(e.target.value)}
        onPressEnter={commit}
        onBlur={commit}
      />
    );
  }

  if (disabled) {
    return value
      ? <Text style={{ fontSize: 13, color: textDisabled }}>{value}</Text>
      : <Text type="secondary" style={{ fontSize: 13 }}>-</Text>;
  }

  return (
    <Text
      style={{
        fontSize: 13,
        color: value ? textDisabled : textPlaceholder,
        cursor: 'pointer',
        borderBottom: '1px dashed transparent',
        transition: 'color 0.15s, border-color 0.15s',
        padding: '2px 0',
      }}
      onMouseEnter={(e) => {
        (e.currentTarget as HTMLElement).style.color = '#1677ff';
        (e.currentTarget as HTMLElement).style.borderBottomColor = '#1677ff';
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLElement).style.color = value ? textDisabled : textPlaceholder;
        (e.currentTarget as HTMLElement).style.borderBottomColor = 'transparent';
      }}
      onClick={() => {
        setDraft(value || '');
        setEditing(true);
      }}
    >
      {value || '点击编辑'}
    </Text>
  );
}

// ─── Inline-Editable Leave-Type Cell ────────────────
function EditableLeaveTypeCell({
  value, disabled, onSave,
}: {
  value: string | null;
  disabled: boolean;
  onSave: (val: string | null) => void;
}) {
  const [open, setOpen] = useState(false);

  if (disabled) {
    return value
      ? <Tag style={{ fontSize: 12, margin: 0 }}>{value}</Tag>
      : <Text type="secondary" style={{ fontSize: 13 }}>-</Text>;
  }

  return (
    <Select
      size="small"
      value={value || undefined}
      placeholder="选择"
      allowClear
      style={{ width: '100%' }}
      open={open}
      onDropdownVisibleChange={(visible) => setOpen(visible)}
      onChange={(val) => {
        setOpen(false);
        onSave(val || null);
      }}
      options={LEAVE_TYPES.map((t) => ({ value: t, label: t }))}
    />
  );
}

// ─── Main Component ──────────────────────────────────
export default function AttendancePage() {
  const [searchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [activeTab, setActiveTab] = useState('register');
  const isDark = useIsDark();

  // ── 暗色适配色值 ──
  const toolbarBg = isDark ? '#141414' : '#fff';
  const toolbarBorder = isDark ? '#303030' : '#f0f0f0';
  const textHeading = isDark ? '#e8e8e8' : '#262626';
  const textDisabled = isDark ? '#8c8c8c' : '#595959';
  const textPlaceholder = isDark ? '#595959' : '#bfbfbf';
  const textLabel = isDark ? '#8c8c8c' : '#8c8c8c';

  // 暗色模式下状态卡片背景映射
  const getStatusBg = (lightBg: string) => {
    if (!isDark) return lightBg;
    const darkMap: Record<string, string> = {
      '#f6ffed': '#1a2e1a',
      '#fff7e6': '#2e2616',
      '#fffbe6': '#2e2a16',
      '#e6f4ff': '#111d2c',
      '#fff1f0': '#2c1616',
    };
    return darkMap[lightBg] || lightBg;
  };

  // ==================== 当日登记状态 ====================
  const [selectedDate, setSelectedDate] = useState(dayjs());
  const [statusFilter, setStatusFilter] = useLocalStorageState<string | undefined>('attendance_register_status', undefined);
  const [page, setPage] = useLocalStorageState('attendance_register_page', 1);
  const [pageSize, setPageSize] = useLocalStorageState('attendance_register_page_size', 20);
  const [leaveModalVisible, setLeaveModalVisible] = useState(false);
  const [editingAttendance, setEditingAttendance] = useState<Attendance | null>(null);
  const dateStr = selectedDate.format('YYYY-MM-DD');

  // ==================== 历史查询状态 ====================
  const [queryDateRange, setQueryDateRange] = useLocalStorageState<[string, string] | null>('attendance_query_range', null);
  const [queryStudentId, setQueryStudentId] = useLocalStorageState<number | undefined>('attendance_query_student', undefined);
  const [queryStatus, setQueryStatus] = useLocalStorageState<string | undefined>('attendance_query_status', undefined);
  const [queryPage, setQueryPage] = useLocalStorageState('attendance_query_page', 1);
  const [queryPageSize, setQueryPageSize] = useLocalStorageState('attendance_query_page_size', 20);

  useEffect(() => {
    const tab = searchParams.get('tab');
    const date = searchParams.get('date');
    const startDate = searchParams.get('start_date');
    const endDate = searchParams.get('end_date');
    const status = searchParams.get('status');

    if (tab === 'register' || tab === 'query') setActiveTab(tab);
    if (date && DATE_PATTERN.test(date)) setSelectedDate(dayjs(date));
    if (startDate && endDate && DATE_PATTERN.test(startDate) && DATE_PATTERN.test(endDate)) {
      setQueryDateRange([startDate, endDate]);
      setQueryPage(1);
    }
    if (status) {
      if (tab === 'register') { setStatusFilter(status); setPage(1); }
      else { setQueryStatus(status); setQueryPage(1); }
    }
  }, [searchParams, setPage, setQueryDateRange, setQueryPage, setQueryStatus, setSelectedDate, setStatusFilter]);

  // ==================== Queries ====================
  const { data: students } = useQuery({
    queryKey: ['allStudents', currentCohort?.id],
    queryFn: () => studentService.getAll(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data: attendanceData, isLoading } = useQuery({
    queryKey: ['attendance', currentCohort?.id, dateStr],
    queryFn: () => attendanceService.getByDate(currentCohort!.id, dateStr),
    enabled: !!currentCohort,
  });

  const queryRangeValue = queryDateRange
    ? [dayjs(queryDateRange[0]), dayjs(queryDateRange[1])] as [dayjs.Dayjs, dayjs.Dayjs]
    : null;
  const qStart = queryDateRange?.[0];
  const qEnd = queryDateRange?.[1];

  const applyRangePreset = (preset: 'week' | 'month' | 'semester') => {
    const now = dayjs();
    if (preset === 'week') {
      setQueryDateRange([now.startOf('week').format('YYYY-MM-DD'), now.endOf('week').format('YYYY-MM-DD')]);
      setQueryPage(1); return;
    }
    if (preset === 'month') {
      setQueryDateRange([now.startOf('month').format('YYYY-MM-DD'), now.endOf('month').format('YYYY-MM-DD')]);
      setQueryPage(1); return;
    }
    const semester = currentCohort?.semester || '';
    const matchYear = semester.match(/(\d{4})/);
    const year = matchYear ? Number(matchYear[1]) : now.year();
    const isSpring = semester.includes('春');
    const start = isSpring ? dayjs(`${year}-02-01`) : dayjs(`${year}-08-01`);
    const end = isSpring ? dayjs(`${year}-07-31`) : dayjs(`${year + 1}-01-31`);
    setQueryDateRange([start.format('YYYY-MM-DD'), end.format('YYYY-MM-DD')]);
    setQueryPage(1);
  };

  const { data: queryResult, isLoading: queryLoading } = useQuery({
    queryKey: ['attendanceQuery', currentCohort?.id, qStart, qEnd, queryStudentId, queryStatus, queryPage, queryPageSize],
    queryFn: () => attendanceService.query(currentCohort!.id, {
      start_date: qStart, end_date: qEnd,
      student_id: queryStudentId, status: queryStatus,
      page: queryPage, page_size: queryPageSize,
    }),
    enabled: !!currentCohort && activeTab === 'query',
  });

  // ==================== Mutations ====================
  const saveMutation = useMutation({
    mutationFn: ({ records }: { records: Array<{ student_id: number; status: string; reason?: string; remark?: string; leave_type?: string; leave_start_date?: string; leave_end_date?: string }> }) =>
      attendanceService.saveAll(currentCohort!.id, dateStr, records),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['attendance'] });
      message.success('考勤保存成功');
    },
    onError: (err: Error) => message.error(err.message),
  });

  const setAllNormalMutation = useMutation({
    mutationFn: () => attendanceService.setAllNormal(currentCohort!.id, dateStr),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['attendance'] });
      message.success('已全部设为正常');
    },
    onError: (err: Error) => message.error(err.message),
  });

  // ==================== 当日登记逻辑 ====================
  const allRecords = (students || []).map((s) => {
    const a = (attendanceData || []).find((r) => r.student_id === s.id);
    return {
      key: s.id,
      student_id: s.id,
      student_name: s.name,
      student_no: s.student_no,
      group_name: s.group_name,
      status: (a?.status || '正常') as AttendanceStatus,
      leave_type: a?.leave_type || null,
      leave_start_date: a?.leave_start_date || null,
      leave_end_date: a?.leave_end_date || null,
      reason: a?.reason || null,
      remark: a?.remark || null,
      attendance_id: a?.id || null,
    };
  });

  const filteredRecords = statusFilter
    ? allRecords.filter((r) => r.status === statusFilter)
    : allRecords;

  const handleStatusChange = (studentId: number, newStatus: string) => {
    if (isReadonly) return;
    if (newStatus === '请假') {
      const record = allRecords.find((r) => r.student_id === studentId);
      setEditingAttendance({
        id: record?.attendance_id || 0,
        cohort_id: currentCohort!.id,
        student_id: studentId,
        attendance_date: dateStr,
        status: '请假',
        leave_type: record?.leave_type || '事假',
        leave_start_date: record?.leave_start_date || dateStr,
        leave_end_date: record?.leave_end_date || dateStr,
        reason: '',
        remark: '',
        created_at: '',
        updated_at: '',
        student_name: record?.student_name,
      });
      setLeaveModalVisible(true);
      return;
    }
    // 始终发送当天全部学生的考勤记录，仅修改目标学生的状态
    // 后端使用 UPSERT，未变动的记录不会产生副作用
    saveMutation.mutate({
      records: allRecords.map((r) => {
        const isTarget = r.student_id === studentId;
        // 只有目标学生且新状态不是「请假」时，才清空请假相关字段
        const shouldClearLeave = isTarget && newStatus !== '请假';
        return {
          student_id: r.student_id,
          status: isTarget ? newStatus : r.status,
          reason: shouldClearLeave ? undefined : (r.reason || undefined),
          remark: shouldClearLeave ? undefined : (r.remark || undefined),
          leave_type: shouldClearLeave ? undefined : (r.leave_type || undefined),
          leave_start_date: shouldClearLeave ? undefined : (r.leave_start_date || undefined),
          leave_end_date: shouldClearLeave ? undefined : (r.leave_end_date || undefined),
        };
      }),
    });
  };

  const handleLeaveConfirm = () => {
    if (!editingAttendance) return;
    saveMutation.mutate({
      records: allRecords.map((r) => ({
        student_id: r.student_id,
        status: r.student_id === editingAttendance.student_id ? '请假' : r.status,
        leave_type: r.student_id === editingAttendance.student_id
          ? (editingAttendance.leave_type || undefined)
          : (r.leave_type || undefined),
        leave_start_date: r.student_id === editingAttendance.student_id
          ? (editingAttendance.leave_start_date || undefined)
          : (r.leave_start_date || undefined),
        leave_end_date: r.student_id === editingAttendance.student_id
          ? (editingAttendance.leave_end_date || undefined)
          : (r.leave_end_date || undefined),
        reason: r.student_id === editingAttendance.student_id
          ? (editingAttendance.reason || undefined)
          : (r.reason || undefined),
        remark: r.student_id === editingAttendance.student_id
          ? (editingAttendance.remark || undefined)
          : (r.remark || undefined),
      })),
    });
    setLeaveModalVisible(false);
    setEditingAttendance(null);
  };

  // 使用 ref 保存 allRecords，避免 handleFieldUpdate 因 allRecords 变化而频繁重建
  const allRecordsRef = useRef(allRecords);
  allRecordsRef.current = allRecords;

  /**
   * 更新指定学生的一条字段（reason / remark / leave_type）
   * 将当天全部登记数据重新提交，后端做 UPSERT
   */
  const handleFieldUpdate = useCallback(
    (studentId: number, field: 'reason' | 'remark' | 'leave_type', value: string | null) => {
      if (isReadonly) return;
      saveMutation.mutate({
        records: allRecordsRef.current.map((r) => ({
          student_id: r.student_id,
          status: r.status,
          reason: r.student_id === studentId && field === 'reason' ? (value || undefined) : (r.reason || undefined),
          remark: r.student_id === studentId && field === 'remark' ? (value || undefined) : (r.remark || undefined),
          leave_type: r.student_id === studentId && field === 'leave_type' ? (value || undefined) : (r.leave_type || undefined),
          leave_start_date: r.leave_start_date || undefined,
          leave_end_date: r.leave_end_date || undefined,
        })),
      });
    },
    [isReadonly, saveMutation],
  );

  // ==================== 导出逻辑 ====================
  const handleExportDaily = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: `考勤_${dateStr}.xlsx`,
      });
      if (filePath) {
        await attendanceService.exportExcel(currentCohort!.id, filePath);
        message.success('导出成功');
      }
    } catch { message.error('导出失败'); }
  };

  const handleExportQuery = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const rangeLabel = qStart && qEnd ? `${qStart}_${qEnd}` : '全部';
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: `考勤查询_${rangeLabel}.xlsx`,
      });
      if (filePath) {
        await attendanceService.exportExcel(currentCohort!.id, filePath, {
          start_date: qStart, end_date: qEnd,
        });
        message.success('导出成功');
      }
    } catch { message.error('导出失败'); }
  };

  if (!currentCohort) return <Empty description="请先选择届次" />;

  // ==================== 当日登记 Stats ====================
  const stats = {
    normal: allRecords.filter((r) => r.status === '正常').length,
    late: allRecords.filter((r) => r.status === '迟到').length,
    early: allRecords.filter((r) => r.status === '早退').length,
    leave: allRecords.filter((r) => r.status === '请假').length,
    absent: allRecords.filter((r) => r.status === '旷课').length,
  };
  const totalStudents = students?.length ?? 0;
  const registeredCount = attendanceData?.length ?? 0;
  const unregisteredCount = totalStudents - registeredCount;

  // ==================== 当日登记 - Register Tab ====================
  const registerTab = (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* ── 1) Toolbar ── */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: 12, flexWrap: 'wrap',
        padding: '8px 16px', background: toolbarBg, borderRadius: 8,
        border: `1px solid ${toolbarBorder}`,
      }}>
        {/* Left: filters */}
        <DatePicker
          value={selectedDate}
          onChange={(d) => d && setSelectedDate(d)}
          allowClear={false}
          size="middle"
          style={{ width: 140 }}
        />
        <Select
          placeholder="状态筛选"
          allowClear
          size="middle"
          style={{ width: 110 }}
          value={statusFilter}
          onChange={(val) => { setStatusFilter(val); setPage(1); }}
          options={ATTENDANCE_STATUSES.map((s) => ({ value: s, label: s }))}
        />

        {/* Center: class summary */}
        <div style={{
          flex: 1, minWidth: 200, display: 'flex', alignItems: 'center', gap: 16,
          padding: '0 12px', borderLeft: `1px solid ${toolbarBorder}`, borderRight: `1px solid ${toolbarBorder}`,
        }}>
          <Text strong style={{ fontSize: 13, color: textHeading }}>
            {currentCohort.cohort_name} {currentCohort.class_name}
          </Text>
          <Space size={12}>
            <span>
              <Text type="secondary" style={{ fontSize: 12 }}>应到</Text>
              <Text strong style={{ fontSize: 13, marginLeft: 4, color: '#1677ff' }}>{totalStudents}</Text>
            </span>
            <span>
              <Text type="secondary" style={{ fontSize: 12 }}>已登记</Text>
              <Badge
                count={registeredCount}
                color={registeredCount === totalStudents ? '#52c41a' : '#faad14'}
                overflowCount={999}
                style={{ marginLeft: 4 }}
              />
            </span>
            {unregisteredCount > 0 && (
              <span>
                <Text type="secondary" style={{ fontSize: 12 }}>未登记</Text>
                <Text style={{ fontSize: 13, marginLeft: 4, color: '#ff4d4f' }}>{unregisteredCount}</Text>
              </span>
            )}
          </Space>
        </div>

        {/* Right: actions */}
        <Space size={8}>
          {!isReadonly && (
            <Button
              type="primary"
              icon={<CheckCircleOutlined />}
              onClick={() => setAllNormalMutation.mutate()}
              loading={setAllNormalMutation.isPending}
              size="middle"
            >
              全部正常
            </Button>
          )}
          <Button
            icon={<DownloadOutlined />}
            onClick={handleExportDaily}
            size="middle"
          >
            导出考勤
          </Button>
        </Space>
      </div>

      {/* ── 2) Today Overview ── */}
      <div style={{ display: 'flex', gap: 10 }}>
        {(['正常', '迟到', '早退', '请假', '旷课'] as const).map((key) => (
          <StatCard
            key={key}
            label={STATUS_LABEL_MAP[key]}
            count={stats[key === '正常' ? 'normal' : key === '迟到' ? 'late' : key === '早退' ? 'early' : key === '请假' ? 'leave' : 'absent']}
            color={STATUS_CONFIG[key].color}
            bg={getStatusBg(STATUS_CONFIG[key].bg)}
            icon={STATUS_CONFIG[key].icon}
            isDark={isDark}
          />
        ))}
      </div>

      {/* ── 3) Student Table ── */}
      <Card
        styles={{ body: { padding: '12px 16px' } }}
        style={{ borderRadius: 8 }}
      >
        <Table
          dataSource={filteredRecords}
          columns={[
            { title: '姓名', dataIndex: 'student_name', key: 'student_name', width: 100 },
            { title: '学号', dataIndex: 'student_no', key: 'student_no', width: 110 },
            { title: '小组', dataIndex: 'group_name', key: 'group_name', width: 80, render: (v: string | null) => v || '-' },
            {
              title: '状态',
              dataIndex: 'status',
              key: 'status',
              width: 110,
              render: (status: AttendanceStatus, record: { student_id: number }) => {
                const cfg = attendanceStatusConfig[status];
                const menuItems = ATTENDANCE_STATUSES.map((s) => {
                  const itemCfg = attendanceStatusConfig[s];
                  return {
                    key: s,
                    label: <Tag color={itemCfg.color} style={{ margin: 0 }}>{itemCfg.label}</Tag>,
                  };
                });

                return isReadonly ? (
                  <Tag color={cfg.color} style={{ margin: 0 }}>{cfg.label}</Tag>
                ) : (
                  <Dropdown
                    menu={{
                      items: menuItems,
                      onClick: ({ key }) => handleStatusChange(record.student_id, key),
                    }}
                    trigger={['click']}
                  >
                    <Tag
                      color={cfg.color}
                      className="attendance-status-tag"
                      onClick={(e) => e.preventDefault()}
                    >
                      {cfg.label}
                      <DownOutlined className="attendance-status-tag-icon" />
                    </Tag>
                  </Dropdown>
                );
              },
            },
            {
              title: '原因',
              dataIndex: 'reason',
              key: 'reason',
              ellipsis: true,
              render: (v: string | null, record: { student_id: number }) => (
                <EditableTextCell
                  value={v}
                  placeholder="输入原因"
                  disabled={isReadonly}
                  onSave={(val) => handleFieldUpdate(record.student_id, 'reason', val || null)}
                  isDark={isDark}
                />
              ),
            },
            {
              title: '请假类型',
              dataIndex: 'leave_type',
              key: 'leave_type',
              width: 100,
              render: (v: string | null, record: { student_id: number; status: string }) => (
                <EditableLeaveTypeCell
                  value={v}
                  disabled={isReadonly || record.status !== '请假'}
                  onSave={(val) => handleFieldUpdate(record.student_id, 'leave_type', val)}
                />
              ),
            },
            {
              title: '备注',
              dataIndex: 'remark',
              key: 'remark',
              ellipsis: true,
              render: (v: string | null, record: { student_id: number }) => (
                <EditableTextCell
                  value={v}
                  placeholder="输入备注"
                  disabled={isReadonly}
                  onSave={(val) => handleFieldUpdate(record.student_id, 'remark', val || null)}
                  isDark={isDark}
                />
              ),
            },
          ]}
          rowKey="key"
          loading={isLoading}
          size="small"
          rowClassName={(record) => {
            if (record.status === '正常') return '';
            return 'attendance-row-abnormal';
          }}
          pagination={{
            current: page,
            pageSize,
            total: filteredRecords.length,
            onChange: (p, ps) => { setPage(p); setPageSize(ps); },
            showSizeChanger: true,
            showTotal: (total, range) => `第 ${range[0]}-${range[1]} 条，共 ${total} 人`,
            size: 'small',
          }}
        />
      </Card>
    </div>
  );

  // ==================== 历史查询 - Query Tab ====================
  const queryColumns = [
    { title: '日期', dataIndex: 'attendance_date', key: 'attendance_date', width: 120 },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 90,
      render: (s: string) => {
        const cfg = STATUS_CONFIG[s];
        return <Tag color={cfg?.color || 'default'} style={{ fontSize: 12, margin: 0 }}>{s}</Tag>;
      },
    },
    { title: '姓名', dataIndex: 'student_name', key: 'student_name', width: 100 },
    { title: '学号', dataIndex: 'student_no', key: 'student_no', width: 110 },
    { title: '请假类型', dataIndex: 'leave_type', key: 'leave_type', width: 100, render: (v: string | null) => v ? <Tag style={{ fontSize: 12, margin: 0 }}>{v}</Tag> : '-' },
    { title: '开始日期', dataIndex: 'leave_start_date', key: 'leave_start_date', width: 110, render: (v: string | null) => v || '-' },
    { title: '结束日期', dataIndex: 'leave_end_date', key: 'leave_end_date', width: 110, render: (v: string | null) => v || '-' },
    { title: '原因', dataIndex: 'reason', key: 'reason', ellipsis: true, render: (v: string | null) => v || '-' },
    { title: '备注', dataIndex: 'remark', key: 'remark', ellipsis: true, render: (v: string | null) => v || '-' },
  ];

  const queryTab = (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {/* Query Toolbar */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: 10, flexWrap: 'wrap',
        padding: '8px 16px', background: toolbarBg, borderRadius: 8,
        border: `1px solid ${toolbarBorder}`,
      }}>
        <RangePicker
          value={queryRangeValue}
          onChange={(dates) => {
            setQueryDateRange(dates ? [dates[0]!.format('YYYY-MM-DD'), dates[1]!.format('YYYY-MM-DD')] : null);
            setQueryPage(1);
          }}
          placeholder={['开始日期', '结束日期']}
          size="middle"
        />
        <Button size="middle" onClick={() => applyRangePreset('week')}>本周</Button>
        <Button size="middle" onClick={() => applyRangePreset('month')}>本月</Button>
        <Button size="middle" onClick={() => applyRangePreset('semester')}>本学期</Button>
        <Select
          placeholder="选择学生"
          allowClear
          showSearch
          size="middle"
          style={{ width: 180 }}
          optionFilterProp="label"
          value={queryStudentId}
          onChange={(val) => { setQueryStudentId(val); setQueryPage(1); }}
          options={(students || []).map((s) => ({ value: s.id, label: `${s.name} (${s.student_no})` }))}
        />
        <Select
          placeholder="状态筛选"
          allowClear
          size="middle"
          style={{ width: 110 }}
          value={queryStatus}
          onChange={(val) => { setQueryStatus(val); setQueryPage(1); }}
          options={ATTENDANCE_STATUSES.map((s) => ({ value: s, label: s }))}
        />
        <Button
          icon={<DownloadOutlined />}
          onClick={handleExportQuery}
          size="middle"
        >
          导出
        </Button>
      </div>

      {/* Query Table */}
      <Card styles={{ body: { padding: '12px 16px' } }} style={{ borderRadius: 8 }}>
        <Table
          dataSource={queryResult?.data || []}
          columns={queryColumns}
          rowKey="id"
          loading={queryLoading}
          size="small"
          rowClassName={(record) => {
            if (record.status === '正常') return '';
            return 'attendance-row-abnormal';
          }}
          pagination={{
            current: queryPage,
            pageSize: queryPageSize,
            total: queryResult?.total || 0,
            onChange: (p, ps) => { setQueryPage(p); setQueryPageSize(ps); },
            showSizeChanger: true,
            showTotal: (total, range) => `第 ${range[0]}-${range[1]} 条，共 ${total} 条记录`,
            size: 'small',
          }}
        />
      </Card>
    </div>
  );

  // ==================== Render ====================
  return (
    <div>
      <div className="page-header">
        <Title level={4} style={{ margin: 0 }}>考勤管理</Title>
      </div>

      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          { key: 'register', label: '当日登记', children: registerTab },
          { key: 'query', label: '历史查询', children: queryTab },
        ]}
      />

      <Modal
        title="请假登记"
        open={leaveModalVisible}
        onOk={handleLeaveConfirm}
        onCancel={() => { setLeaveModalVisible(false); setEditingAttendance(null); }}
      >
        <p>学生：{editingAttendance?.student_name}</p>
        <p>日期：{dateStr}</p>
        <Select
          placeholder="请选择请假类型"
          style={{ width: '100%', marginBottom: 8 }}
          value={editingAttendance?.leave_type || '事假'}
          onChange={(value) => setEditingAttendance((prev) => prev ? { ...prev, leave_type: value } : null)}
          options={LEAVE_TYPES.map((item) => ({ value: item, label: item }))}
        />
        <Space style={{ width: '100%', marginBottom: 8 }}>
          <DatePicker
            style={{ width: '100%' }}
            value={editingAttendance?.leave_start_date ? dayjs(editingAttendance.leave_start_date) : dayjs(dateStr)}
            onChange={(value) =>
              setEditingAttendance((prev) => prev ? { ...prev, leave_start_date: value?.format('YYYY-MM-DD') || dateStr } : null)
            }
          />
          <DatePicker
            style={{ width: '100%' }}
            value={editingAttendance?.leave_end_date ? dayjs(editingAttendance.leave_end_date) : dayjs(dateStr)}
            onChange={(value) =>
              setEditingAttendance((prev) => prev ? { ...prev, leave_end_date: value?.format('YYYY-MM-DD') || dateStr } : null)
            }
          />
        </Space>
        <Input.TextArea
          placeholder="请输入请假原因"
          rows={3}
          value={editingAttendance?.reason || ''}
          onChange={(e) => setEditingAttendance((prev) => prev ? { ...prev, reason: e.target.value } : null)}
          style={{ marginBottom: 8 }}
        />
        <Input.TextArea
          placeholder="备注（可选）"
          rows={2}
          value={editingAttendance?.remark || ''}
          onChange={(e) => setEditingAttendance((prev) => prev ? { ...prev, remark: e.target.value } : null)}
        />
      </Modal>
    </div>
  );
}

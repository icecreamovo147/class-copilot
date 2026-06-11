import { useState } from 'react';
import { Card, Table, Button, DatePicker, Select, Tag, message, Typography, Space, Empty, Descriptions, Modal, Input, Tabs } from 'antd';
import { CheckCircleOutlined, DownloadOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { attendanceService, studentService } from '@/services';
import type { Attendance, AttendanceStatus } from '@/types';
import { ATTENDANCE_STATUSES } from '@/types';
import dayjs from 'dayjs';

const { Title } = Typography;
const { RangePicker } = DatePicker;

const statusColors: Record<string, string> = {
  '正常': 'green',
  '迟到': 'orange',
  '早退': 'gold',
  '请假': 'blue',
  '旷课': 'red',
};

export default function AttendancePage() {
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [activeTab, setActiveTab] = useState('register');

  // ==================== 当日登记状态 ====================
  const [selectedDate, setSelectedDate] = useState(dayjs());
  const [statusFilter, setStatusFilter] = useState<string | undefined>();
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [leaveModalVisible, setLeaveModalVisible] = useState(false);
  const [editingAttendance, setEditingAttendance] = useState<Attendance | null>(null);
  const dateStr = selectedDate.format('YYYY-MM-DD');

  // ==================== 历史查询状态 ====================
  const [queryDateRange, setQueryDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs] | null>(null);
  const [queryStudentId, setQueryStudentId] = useState<number | undefined>();
  const [queryStatus, setQueryStatus] = useState<string | undefined>();
  const [queryPage, setQueryPage] = useState(1);
  const [queryPageSize, setQueryPageSize] = useState(20);

  // ==================== 当日登记数据 ====================
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

  // ==================== 历史查询数据 ====================
  const qStart = queryDateRange?.[0]?.format('YYYY-MM-DD');
  const qEnd = queryDateRange?.[1]?.format('YYYY-MM-DD');

  const { data: queryResult, isLoading: queryLoading } = useQuery({
    queryKey: ['attendanceQuery', currentCohort?.id, qStart, qEnd, queryStudentId, queryStatus, queryPage, queryPageSize],
    queryFn: () => attendanceService.query(currentCohort!.id, {
      start_date: qStart,
      end_date: qEnd,
      student_id: queryStudentId,
      status: queryStatus,
      page: queryPage,
      page_size: queryPageSize,
    }),
    enabled: !!currentCohort && activeTab === 'query',
  });

  // ==================== Mutations ====================
  const saveMutation = useMutation({
    mutationFn: ({ records }: { records: Array<{ student_id: number; status: string; reason?: string; remark?: string }> }) =>
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
    const attendance = (attendanceData || []).find((a) => a.student_id === s.id);
    return {
      key: s.id,
      student_id: s.id,
      student_name: s.name,
      student_no: s.student_no,
      group_name: s.group_name,
      status: attendance?.status || '正常' as AttendanceStatus,
      reason: attendance?.reason || null,
      remark: attendance?.remark || null,
      attendance_id: attendance?.id || null,
    };
  });

  const filteredRecords = statusFilter
    ? allRecords.filter((r) => r.status === statusFilter)
    : allRecords;

  const handleStatusChange = async (studentId: number, newStatus: string) => {
    if (isReadonly) return;
    if (newStatus === '请假') {
      const record = allRecords.find((r) => r.student_id === studentId);
      setEditingAttendance({
        id: record?.attendance_id || 0,
        cohort_id: currentCohort!.id,
        student_id: studentId,
        attendance_date: dateStr,
        status: '请假',
        reason: '',
        remark: '',
        created_at: '',
        updated_at: '',
        student_name: record?.student_name,
      });
      setLeaveModalVisible(true);
      return;
    }
    saveMutation.mutate({
      records: allRecords
        .filter((r) => r.status !== newStatus || r.student_id === studentId)
        .map((r) => ({
          student_id: r.student_id,
          status: r.student_id === studentId ? newStatus : r.status,
          reason: r.reason || undefined,
          remark: r.remark || undefined,
        })),
    });
  };

  const handleLeaveConfirm = () => {
    if (!editingAttendance) return;
    saveMutation.mutate({
      records: allRecords.map((r) => ({
        student_id: r.student_id,
        status: r.student_id === editingAttendance.student_id ? '请假' : r.status,
        reason: r.student_id === editingAttendance.student_id ? editingAttendance.reason || undefined : undefined,
        remark: r.student_id === editingAttendance.student_id ? editingAttendance.remark || undefined : undefined,
      })),
    });
    setLeaveModalVisible(false);
    setEditingAttendance(null);
  };

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
    } catch {
      message.error('导出失败');
    }
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
          start_date: qStart,
          end_date: qEnd,
        });
        message.success('导出成功');
      }
    } catch {
      message.error('导出失败');
    }
  };

  if (!currentCohort) return <Empty description="请先选择届次" />;

  // ==================== 当日登记渲染 ====================
  const stats = {
    normal: allRecords.filter((r) => r.status === '正常').length,
    late: allRecords.filter((r) => r.status === '迟到').length,
    early: allRecords.filter((r) => r.status === '早退').length,
    leave: allRecords.filter((r) => r.status === '请假').length,
    absent: allRecords.filter((r) => r.status === '旷课').length,
  };

  const registerColumns = [
    { title: '姓名', dataIndex: 'student_name', key: 'student_name' },
    { title: '学号', dataIndex: 'student_no', key: 'student_no' },
    { title: '小组', dataIndex: 'group_name', key: 'group_name' },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: AttendanceStatus, record: { student_id: number }) => (
        <Select
          value={status}
          style={{ width: 100 }}
          disabled={isReadonly}
          onChange={(val) => handleStatusChange(record.student_id, val)}
          options={ATTENDANCE_STATUSES.map((s) => ({
            value: s,
            label: <Tag color={statusColors[s]}>{s}</Tag>,
          }))}
        />
      ),
    },
    { title: '原因', dataIndex: 'reason', key: 'reason', ellipsis: true },
    { title: '备注', dataIndex: 'remark', key: 'remark', ellipsis: true },
  ];

  const registerTab = (
    <>
      <Card size="small" style={{ marginBottom: 16 }}>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Space wrap>
            <DatePicker value={selectedDate} onChange={(d) => d && setSelectedDate(d)} allowClear={false} />
            <Select
              placeholder="状态筛选"
              allowClear
              style={{ width: 120 }}
              onChange={(val) => { setStatusFilter(val); setPage(1); }}
              options={ATTENDANCE_STATUSES.map((s) => ({ value: s, label: s }))}
            />
          </Space>
          <Space>
            <Descriptions size="small" column={5} bordered>
              <Descriptions.Item label="正常">
                <span style={{ color: '#52c41a', fontWeight: 'bold' }}>{stats.normal}</span>
              </Descriptions.Item>
              <Descriptions.Item label="迟到">
                <span style={{ color: '#faad14', fontWeight: 'bold' }}>{stats.late}</span>
              </Descriptions.Item>
              <Descriptions.Item label="早退">
                <span style={{ color: '#faad14', fontWeight: 'bold' }}>{stats.early}</span>
              </Descriptions.Item>
              <Descriptions.Item label="请假">
                <span style={{ color: '#1677ff', fontWeight: 'bold' }}>{stats.leave}</span>
              </Descriptions.Item>
              <Descriptions.Item label="旷课">
                <span style={{ color: '#ff4d4f', fontWeight: 'bold' }}>{stats.absent}</span>
              </Descriptions.Item>
            </Descriptions>
          </Space>
        </Space>
      </Card>

      <Space style={{ marginBottom: 16 }}>
        {!isReadonly && (
          <Button
            type="primary"
            icon={<CheckCircleOutlined />}
            onClick={() => setAllNormalMutation.mutate()}
            loading={setAllNormalMutation.isPending}
          >
            全部正常
          </Button>
        )}
        <Button icon={<DownloadOutlined />} onClick={handleExportDaily}>
          导出考勤
        </Button>
      </Space>

      <Card>
        <Table
          dataSource={filteredRecords}
          columns={registerColumns}
          rowKey="key"
          loading={isLoading}
          pagination={{
            current: page,
            pageSize,
            total: filteredRecords.length,
            onChange: (p, ps) => { setPage(p); setPageSize(ps); },
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 人`,
          }}
        />
      </Card>
    </>
  );

  // ==================== 历史查询渲染 ====================
  const queryColumns = [
    { title: '日期', dataIndex: 'attendance_date', key: 'attendance_date', width: 120 },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (s: string) => <Tag color={statusColors[s] || 'default'}>{s}</Tag>,
    },
    { title: '学生姓名', dataIndex: 'student_name', key: 'student_name' },
    { title: '学号', dataIndex: 'student_no', key: 'student_no' },
    { title: '原因', dataIndex: 'reason', key: 'reason', ellipsis: true },
    { title: '备注', dataIndex: 'remark', key: 'remark', ellipsis: true },
  ];

  const queryTab = (
    <>
      <Card size="small" style={{ marginBottom: 16 }}>
        <Space wrap>
          <RangePicker
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            value={queryDateRange as any}
            onChange={(dates) => {
              setQueryDateRange(dates as [dayjs.Dayjs, dayjs.Dayjs] | null);
              setQueryPage(1);
            }}
            placeholder={['开始日期', '结束日期']}
          />
          <Select
            placeholder="选择学生"
            allowClear
            showSearch
            style={{ width: 180 }}
            optionFilterProp="label"
            value={queryStudentId}
            onChange={(val) => { setQueryStudentId(val); setQueryPage(1); }}
            options={(students || []).map((s) => ({ value: s.id, label: `${s.name} (${s.student_no})` }))}
          />
          <Select
            placeholder="状态筛选"
            allowClear
            style={{ width: 120 }}
            value={queryStatus}
            onChange={(val) => { setQueryStatus(val); setQueryPage(1); }}
            options={ATTENDANCE_STATUSES.map((s) => ({ value: s, label: s }))}
          />
          <Button
            icon={<DownloadOutlined />}
            onClick={handleExportQuery}
            type="primary"
            ghost
          >
            导出当前筛选结果
          </Button>
        </Space>
      </Card>

      <Card>
        <Table
          dataSource={queryResult?.data || []}
          columns={queryColumns}
          rowKey="id"
          loading={queryLoading}
          pagination={{
            current: queryPage,
            pageSize: queryPageSize,
            total: queryResult?.total || 0,
            onChange: (p, ps) => { setQueryPage(p); setQueryPageSize(ps); },
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条记录`,
          }}
        />
      </Card>
    </>
  );

  return (
    <div>
      <div className="page-header">
        <Title level={4}>考勤管理</Title>
      </div>

      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: 'register',
            label: '当日登记',
            children: registerTab,
          },
          {
            key: 'query',
            label: '历史查询',
            children: queryTab,
          },
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

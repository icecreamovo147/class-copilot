import { useState } from 'react';
import { Card, Table, Button, Modal, Form, Input, Select, DatePicker, Tabs, Space, Tag, message, Typography, Popconfirm, Row, Col } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { noticeService, dutyService, behaviorService, studentService } from '@/services';
import type { Notice, Duty, BehaviorRecord } from '@/types';
import { BEHAVIOR_TYPES } from '@/types';
import dayjs from 'dayjs';

const { Title } = Typography;
const { TextArea } = Input;

// ==================== 通知管理子组件 ====================
function NoticeTab() {
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [modalVisible, setModalVisible] = useState(false);
  const [editing, setEditing] = useState<Notice | null>(null);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [form] = Form.useForm();

  const { data, isLoading } = useQuery({
    queryKey: ['notices', currentCohort?.id, page, search],
    queryFn: () => noticeService.list(currentCohort!.id, { page, page_size: 20, search: search || undefined }),
    enabled: !!currentCohort,
  });

  const createMutation = useMutation({
    mutationFn: (data: Partial<Notice>) => noticeService.create(data),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['notices'] }); message.success('通知创建成功'); setModalVisible(false); form.resetFields(); },
    onError: (err: Error) => message.error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<Notice> }) => noticeService.update(id, data),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['notices'] }); message.success('通知更新成功'); setModalVisible(false); setEditing(null); },
    onError: (err: Error) => message.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => noticeService.delete(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['notices'] }); message.success('通知已删除'); },
    onError: (err: Error) => message.error(err.message),
  });

  const handleSubmit = async () => {
    const values = await form.validateFields();
    values.cohort_id = currentCohort!.id;
    values.publish_date = values.publish_date?.format('YYYY-MM-DD') || dayjs().format('YYYY-MM-DD');
    if (editing) { updateMutation.mutate({ id: editing.id, data: values }); }
    else { createMutation.mutate(values); }
  };

  const columns = [
    { title: '标题', dataIndex: 'title', key: 'title' },
    { title: '发布日期', dataIndex: 'publish_date', key: 'publish_date', width: 120 },
    { title: '内容', dataIndex: 'content', key: 'content', ellipsis: true },
    {
      title: '操作', key: 'action', width: 160,
      render: (_: unknown, record: Notice) => (
        <Space>
          <Button type="link" icon={<EditOutlined />} disabled={isReadonly} onClick={() => { setEditing(record); form.setFieldsValue(record); setModalVisible(true); }}>编辑</Button>
          <Popconfirm title="确定删除？" onConfirm={() => deleteMutation.mutate(record.id)}>
            <Button type="link" icon={<DeleteOutlined />} disabled={isReadonly} danger>删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Space style={{ marginBottom: 16 }} wrap>
        {!isReadonly && (
          <Button type="primary" icon={<PlusOutlined />} onClick={() => { setEditing(null); form.resetFields(); setModalVisible(true); }}>
            新增通知
          </Button>
        )}
        <Input
          placeholder="搜索通知标题/内容"
          allowClear
          style={{ width: 220 }}
          value={search}
          onChange={(e) => { setSearch(e.target.value); setPage(1); }}
        />
      </Space>
      <Table dataSource={data?.data || []} columns={columns} rowKey="id" loading={isLoading} pagination={{ current: page, total: data?.total || 0, onChange: setPage }} size="small" />
      <Modal title={editing ? '编辑通知' : '新增通知'} open={modalVisible} onOk={handleSubmit} onCancel={() => { setModalVisible(false); setEditing(null); }} confirmLoading={createMutation.isPending || updateMutation.isPending}>
        <Form form={form} layout="vertical">
          <Form.Item name="title" label="标题" rules={[{ required: true }]}><Input /></Form.Item>
          <Form.Item name="publish_date" label="发布日期"><DatePicker style={{ width: '100%' }} /></Form.Item>
          <Form.Item name="content" label="正文"><TextArea rows={4} /></Form.Item>
          <Form.Item name="remark" label="备注"><TextArea rows={2} /></Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

// ==================== 值日安排子组件 ====================
function DutyTab() {
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [modalVisible, setModalVisible] = useState(false);
  const [editing, setEditing] = useState<Duty | null>(null);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [dutyStatus, setDutyStatus] = useState<string | undefined>();
  const [form] = Form.useForm();

  const { data: students } = useQuery({
    queryKey: ['allStudents', currentCohort?.id],
    queryFn: () => studentService.getAll(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data, isLoading } = useQuery({
    queryKey: ['duties', currentCohort?.id, page, search, dutyStatus],
    queryFn: () => dutyService.list(currentCohort!.id, { page, page_size: 20, search: search || undefined }),
    enabled: !!currentCohort,
  });

  // 前端按状态进一步筛选
  const filteredData = dutyStatus
    ? (data?.data || []).filter((d) => d.status === dutyStatus)
    : (data?.data || []);

  const createMutation = useMutation({
    mutationFn: (data: Partial<Duty>) => dutyService.create(data),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['duties'] }); message.success('值日安排已创建'); setModalVisible(false); form.resetFields(); },
    onError: (err: Error) => message.error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<Duty> }) => dutyService.update(id, data),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['duties'] }); message.success('值日安排已更新'); setModalVisible(false); setEditing(null); },
    onError: (err: Error) => message.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => dutyService.delete(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['duties'] }); message.success('值日安排已删除'); },
    onError: (err: Error) => message.error(err.message),
  });

  const handleSubmit = async () => {
    const values = await form.validateFields();
    values.cohort_id = currentCohort!.id;
    values.duty_date = values.duty_date?.format('YYYY-MM-DD');
    if (editing) { updateMutation.mutate({ id: editing.id, data: values }); }
    else { createMutation.mutate(values); }
  };

  const columns = [
    { title: '日期', dataIndex: 'duty_date', key: 'duty_date', width: 110 },
    { title: '学生', dataIndex: 'student_name', key: 'student_name' },
    { title: '小组', dataIndex: 'group_name', key: 'group_name' },
    { title: '内容', dataIndex: 'duty_content', key: 'duty_content' },
    { title: '状态', dataIndex: 'status', key: 'status', render: (s: string) => <Tag color={s === '已完成' ? 'green' : 'default'}>{s}</Tag> },
    {
      title: '操作', key: 'action', width: 160,
      render: (_: unknown, record: Duty) => (
        <Space>
          <Button type="link" icon={<EditOutlined />} disabled={isReadonly} onClick={() => { setEditing(record); form.setFieldsValue(record); setModalVisible(true); }}>编辑</Button>
          <Popconfirm title="确定删除？" onConfirm={() => deleteMutation.mutate(record.id)}>
            <Button type="link" icon={<DeleteOutlined />} disabled={isReadonly} danger>删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Space style={{ marginBottom: 16 }} wrap>
        {!isReadonly && (
          <Button type="primary" icon={<PlusOutlined />} onClick={() => { setEditing(null); form.resetFields(); setModalVisible(true); }}>
            新增值日
          </Button>
        )}
        <Input
          placeholder="搜索小组/内容"
          allowClear
          style={{ width: 180 }}
          value={search}
          onChange={(e) => { setSearch(e.target.value); setPage(1); }}
        />
        <Select
          placeholder="状态筛选"
          allowClear
          style={{ width: 120 }}
          value={dutyStatus}
          onChange={setDutyStatus}
          options={[
            { value: '未完成', label: '未完成' },
            { value: '已完成', label: '已完成' },
          ]}
        />
      </Space>
      <Table dataSource={filteredData} columns={columns} rowKey="id" loading={isLoading} pagination={{ current: page, total: data?.total || 0, onChange: setPage }} size="small" />
      <Modal title={editing ? '编辑值日' : '新增值日'} open={modalVisible} onOk={handleSubmit} onCancel={() => { setModalVisible(false); setEditing(null); }}>
        <Form form={form} layout="vertical">
          <Form.Item name="duty_date" label="日期" rules={[{ required: true }]}><DatePicker style={{ width: '100%' }} /></Form.Item>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="student_id" label="学生">
                <Select
                  showSearch
                  allowClear
                  placeholder="选择学生"
                  optionFilterProp="label"
                  options={(students || []).map((s) => ({ value: s.id, label: `${s.name} (${s.student_no})` }))}
                />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="group_name" label="小组"><Input /></Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="status" label="状态">
                <Select options={[{ value: '未完成', label: '未完成' }, { value: '已完成', label: '已完成' }]} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="duty_content" label="值日内容"><TextArea rows={2} /></Form.Item>
          <Form.Item name="remark" label="备注"><TextArea rows={2} /></Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

// ==================== 奖惩记录子组件 ====================
function BehaviorTab() {
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [modalVisible, setModalVisible] = useState(false);
  const [page, setPage] = useState(1);
  const [filterType, setFilterType] = useState<string | undefined>();
  const [filterStudentId, setFilterStudentId] = useState<number | undefined>();
  const [form] = Form.useForm();

  const { data: students } = useQuery({
    queryKey: ['allStudents', currentCohort?.id],
    queryFn: () => studentService.getAll(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data, isLoading } = useQuery({
    queryKey: ['behaviors', currentCohort?.id, page, filterType, filterStudentId],
    queryFn: () => behaviorService.list(currentCohort!.id, { page, page_size: 20, type: filterType, student_id: filterStudentId }),
    enabled: !!currentCohort,
  });

  const createMutation = useMutation({
    mutationFn: (data: Partial<BehaviorRecord>) => behaviorService.create(data),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['behaviors'] }); message.success('记录已创建'); setModalVisible(false); form.resetFields(); },
    onError: (err: Error) => message.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => behaviorService.delete(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['behaviors'] }); message.success('记录已删除'); },
    onError: (err: Error) => message.error(err.message),
  });

  const handleSubmit = async () => {
    const values = await form.validateFields();
    values.cohort_id = currentCohort!.id;
    values.record_date = values.record_date?.format('YYYY-MM-DD') || dayjs().format('YYYY-MM-DD');
    createMutation.mutate(values);
  };

  const typeColors: Record<string, string> = { '表扬': 'green', '违纪': 'red', '加分': 'blue', '减分': 'orange' };

  const columns = [
    { title: '日期', dataIndex: 'record_date', key: 'record_date', width: 110 },
    { title: '学生', dataIndex: 'student_name', key: 'student_name' },
    { title: '学号', dataIndex: 'student_no', key: 'student_no', width: 80 },
    { title: '类型', dataIndex: 'type', key: 'type', width: 80, render: (t: string) => <Tag color={typeColors[t] || 'default'}>{t}</Tag> },
    { title: '标题', dataIndex: 'title', key: 'title' },
    { title: '分值', dataIndex: 'score', key: 'score', width: 80, render: (v: number) => v !== 0 ? <span style={{ color: v > 0 ? '#52c41a' : '#ff4d4f' }}>{v > 0 ? `+${v}` : v}</span> : '-' },
    {
      title: '操作', key: 'action', width: 80,
      render: (_: unknown, record: BehaviorRecord) => (
        <Popconfirm title="确定删除？" onConfirm={() => deleteMutation.mutate(record.id)}>
          <Button type="link" icon={<DeleteOutlined />} disabled={isReadonly} danger>删除</Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <div>
      <Space style={{ marginBottom: 16 }} wrap>
        {!isReadonly && (
          <Button type="primary" icon={<PlusOutlined />} onClick={() => { form.resetFields(); setModalVisible(true); }}>
            新增奖惩记录
          </Button>
        )}
        <Select
          placeholder="类型筛选"
          allowClear
          style={{ width: 120 }}
          value={filterType}
          onChange={(val) => { setFilterType(val); setPage(1); }}
          options={BEHAVIOR_TYPES.map((t) => ({ value: t, label: t }))}
        />
        <Select
          placeholder="选择学生"
          allowClear
          showSearch
          style={{ width: 180 }}
          optionFilterProp="label"
          value={filterStudentId}
          onChange={(val) => { setFilterStudentId(val); setPage(1); }}
          options={(students || []).map((s) => ({ value: s.id, label: `${s.name} (${s.student_no})` }))}
        />
      </Space>
      <Table dataSource={data?.data || []} columns={columns} rowKey="id" loading={isLoading} pagination={{ current: page, total: data?.total || 0, onChange: setPage }} size="small" />
      <Modal title="新增奖惩记录" open={modalVisible} onOk={handleSubmit} onCancel={() => setModalVisible(false)} confirmLoading={createMutation.isPending}>
        <Form form={form} layout="vertical">
          <Form.Item name="title" label="标题" rules={[{ required: true, message: '请输入奖惩标题' }]}><Input placeholder="如：课堂表现优秀" /></Form.Item>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="type" label="类型" rules={[{ required: true, message: '请选择类型' }]}>
                <Select options={BEHAVIOR_TYPES.map((t) => ({ value: t, label: t }))} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="score" label="分值">
                <Input type="number" placeholder="正数加分，负数控分" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="student_id" label="学生" rules={[{ required: true, message: '请选择学生' }]}>
            <Select
              showSearch
              placeholder="搜索并选择学生"
              optionFilterProp="label"
              options={(students || []).map((s) => ({ value: s.id, label: `${s.name} (${s.student_no})` }))}
            />
          </Form.Item>
          <Form.Item name="record_date" label="日期"><DatePicker style={{ width: '100%' }} /></Form.Item>
          <Form.Item name="description" label="说明"><TextArea rows={3} /></Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

export default function AffairsPage() {
  const tabItems = [
    { key: 'notice', label: '通知记录', children: <NoticeTab /> },
    { key: 'duty', label: '值日安排', children: <DutyTab /> },
    { key: 'behavior', label: '奖惩记录', children: <BehaviorTab /> },
  ];

  return (
    <div>
      <div className="page-header">
        <Title level={4}>班级事务</Title>
      </div>
      <Card>
        <Tabs items={tabItems} />
      </Card>
    </div>
  );
}

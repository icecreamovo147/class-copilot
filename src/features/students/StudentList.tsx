import { useEffect, useState } from 'react';
import {
  Card, Table, Button, Modal, Form, Input, Select, Space, Tag, message, Popconfirm, Typography, Row, Col, Spin, Alert,
} from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined, UploadOutlined, DownloadOutlined, EyeOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { studentService } from '@/services';
import { useLocalStorageState } from '@/hooks/useLocalStorageState';
import { useKeyboardShortcut } from '@/hooks/useKeyboardShortcut';
import type { Student } from '@/types';
import type { ColumnsType } from 'antd/es/table';

const { Title } = Typography;

export default function StudentList() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [modalVisible, setModalVisible] = useState(false);
  const [editingStudent, setEditingStudent] = useState<Student | null>(null);
  const [searchText, setSearchText] = useLocalStorageState('student_list_search', '');
  const [genderFilter, setGenderFilter] = useLocalStorageState<string | undefined>('student_list_gender', undefined);
  const [groupFilter, setGroupFilter] = useLocalStorageState<string | undefined>('student_list_group', undefined);
  const [statusFilter, setStatusFilter] = useLocalStorageState<string | undefined>('student_list_status', undefined);
  const [focusFilter, setFocusFilter] = useLocalStorageState<string | undefined>('student_list_focus', undefined);
  const [page, setPage] = useLocalStorageState('student_list_page', 1);
  const [pageSize, setPageSize] = useLocalStorageState('student_list_page_size', 20);
  const [form] = Form.useForm();

  const { data, isLoading } = useQuery({
    queryKey: ['students', currentCohort?.id, page, pageSize, searchText, genderFilter, groupFilter, statusFilter],
    queryFn: () =>
      studentService.list(currentCohort!.id, {
        page,
        page_size: pageSize,
        search: searchText || undefined,
        gender: genderFilter,
        group_name: groupFilter,
        status: statusFilter,
        is_focus: focusFilter === '1' ? true : focusFilter === '0' ? false : undefined,
      }),
    enabled: !!currentCohort,
  });

  useEffect(() => {
    const focus = searchParams.get('focus');
    const status = searchParams.get('status');
    if (focus === '1' || focus === '0') {
      setFocusFilter(focus);
    }
    if (status) {
      setStatusFilter(status);
    }
    if (focus || status) {
      setPage(1);
    }
  }, [searchParams, setFocusFilter, setPage, setStatusFilter]);

  const createMutation = useMutation({
    mutationFn: (data: Partial<Student>) => studentService.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['students'] });
      message.success('学生添加成功');
      setModalVisible(false);
      form.resetFields();
    },
    onError: (err: Error) => message.error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<Student> }) => studentService.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['students'] });
      message.success('学生信息更新成功');
      setModalVisible(false);
      setEditingStudent(null);
    },
    onError: (err: Error) => message.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => studentService.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['students'] });
      message.success('学生已删除');
    },
    onError: (err: Error) => message.error(err.message),
  });

  const handleCreate = () => {
    if (isReadonly) return;
    setEditingStudent(null);
    form.resetFields();
    setModalVisible(true);
  };

  const handleEdit = (student: Student) => {
    if (isReadonly) return;
    setEditingStudent(student);
    form.setFieldsValue(student);
    setModalVisible(true);
  };

  const handleSubmit = async () => {
    const values = await form.validateFields();
    values.cohort_id = currentCohort!.id;
    if (editingStudent) {
      updateMutation.mutate({ id: editingStudent.id, data: values });
    } else {
      createMutation.mutate(values);
    }
  };

  // 导入预览相关状态
  const [previewVisible, setPreviewVisible] = useState(false);
  const [previewData, setPreviewData] = useState<{
    total_rows: number; valid_rows: number; error_rows: number;
    rows: Array<Record<string, unknown>>; errors: string[];
  } | null>(null);
  const [importFilePath, setImportFilePath] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  // 选择文件 → 预览解析结果
  const handleImport = async () => {
    try {
      const selected = await import('@tauri-apps/plugin-dialog').then((m) =>
        m.open({
          multiple: false,
          filters: [{ name: 'Excel', extensions: ['xlsx', 'xls'] }],
        })
      );
      if (!selected || typeof selected !== 'string') return;
      setImportFilePath(selected);
      setPreviewLoading(true);
      const preview = await studentService.previewExcel(currentCohort!.id, selected);
      setPreviewData(preview);
      setPreviewVisible(true);
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '读取文件失败');
    } finally {
      setPreviewLoading(false);
    }
  };

  // 确认导入
  const handleConfirmImport = async () => {
    if (!importFilePath) return;
    try {
      const result = await studentService.importExcel(currentCohort!.id, importFilePath);
      if (result.errors && result.errors.length > 0) {
        message.warning(`成功导入 ${result.success} 条，${result.errors.length} 条失败`);
      } else {
        message.success(`成功导入 ${result.success} 条记录`);
      }
      queryClient.invalidateQueries({ queryKey: ['students'] });
      queryClient.invalidateQueries({ queryKey: ['allStudents'] });
      setPreviewVisible(false);
      setPreviewData(null);
      setImportFilePath(null);
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '导入失败');
    }
  };

  const handleExport = async () => {
    try {
      const filePath = await import('@tauri-apps/plugin-dialog').then((m) =>
        m.save({
          filters: [{ name: 'Excel', extensions: ['xlsx'] }],
          defaultPath: `学生名单_${currentCohort?.cohort_name}_${currentCohort?.class_name}.xlsx`,
        })
      );
      if (filePath) {
        await studentService.exportExcel(currentCohort!.id, filePath);
        message.success('导出成功');
      }
    } catch {
      message.error('导出失败');
    }
  };

  const columns: ColumnsType<Student> = [
    { title: '姓名', dataIndex: 'name', key: 'name', width: 120 },
    { title: '学号', dataIndex: 'student_no', key: 'student_no', width: 120 },
    { title: '性别', dataIndex: 'gender', key: 'gender', width: 80 },
    { title: '家长电话', dataIndex: 'parent_phone', key: 'parent_phone', width: 140 },
    { title: '小组', dataIndex: 'group_name', key: 'group_name', width: 100 },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: string) => (status === '正常' ? <Tag color="green">正常</Tag> : <Tag color="orange">{status}</Tag>),
    },
    {
      title: '重点关注',
      dataIndex: 'is_focus',
      key: 'is_focus',
      width: 100,
      render: (focus: boolean) => (focus ? <Tag color="red">是</Tag> : null),
    },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Student) => (
        <Space>
          <Button type="link" icon={<EyeOutlined />} onClick={() => navigate(`/students/${record.id}`)}>
            详情
          </Button>
          <Button type="link" icon={<EditOutlined />} disabled={isReadonly} onClick={() => handleEdit(record)}>
            编辑
          </Button>
          <Popconfirm title="确定删除该学生？历史记录将保留。" onConfirm={() => deleteMutation.mutate(record.id)}>
            <Button type="link" icon={<DeleteOutlined />} disabled={isReadonly} danger>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];
  // 获取所有学生以便提取分组列表
  const { data: allStudents } = useQuery({
    queryKey: ['allStudents', currentCohort?.id],
    queryFn: () => studentService.getAll(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const groups = [...new Set((allStudents || []).map((s) => s.group_name).filter(Boolean))];

  useKeyboardShortcut('n', handleCreate, { ctrlOrMeta: true, enabled: !isReadonly });
  useKeyboardShortcut('i', handleImport, { ctrlOrMeta: true, enabled: !isReadonly });
  useKeyboardShortcut('e', handleExport, { ctrlOrMeta: true });

  return (
    <div>
      <div className="page-header">
        <Title level={4}>学生信息管理</Title>
        <Space>
          {!isReadonly && (
            <>
              <Button type="primary" icon={<PlusOutlined />} onClick={handleCreate}>
                新增学生
              </Button>
              <Button icon={<UploadOutlined />} onClick={handleImport}>
                导入 Excel
              </Button>
            </>
          )}
          <Button icon={<DownloadOutlined />} onClick={handleExport}>
            导出 Excel
          </Button>
        </Space>
      </div>

      <Card>
        <div className="filter-bar">
          <Input.Search
            placeholder="姓名/学号"
            allowClear
            value={searchText}
            onChange={(event) => setSearchText(event.target.value)}
            onSearch={(value) => {
              setSearchText(value);
              setPage(1);
            }}
            style={{ width: 200 }}
          />
          <Select
            placeholder="性别"
            allowClear
            style={{ width: 100 }}
            value={genderFilter}
            onChange={(value) => {
              setGenderFilter(value);
              setPage(1);
            }}
            options={[{ value: '男', label: '男' }, { value: '女', label: '女' }]}
          />
          <Select
            placeholder="小组"
            allowClear
            style={{ width: 120 }}
            value={groupFilter}
            onChange={(value) => {
              setGroupFilter(value);
              setPage(1);
            }}
            options={groups.map((g) => ({ value: g!, label: g }))}
          />
          <Select
            placeholder="状态"
            allowClear
            style={{ width: 120 }}
            value={statusFilter}
            onChange={(value) => {
              setStatusFilter(value);
              setPage(1);
            }}
            options={[
              { value: '正常', label: '正常' },
              { value: '休学', label: '休学' },
              { value: '退学', label: '退学' },
            ]}
          />
          <Select
            placeholder="重点关注"
            allowClear
            style={{ width: 140 }}
            value={focusFilter}
            onChange={(value) => {
              setFocusFilter(value);
              setPage(1);
            }}
            options={[
              { value: '1', label: '仅重点关注' },
              { value: '0', label: '仅非重点' },
            ]}
          />
        </div>

        <Table
          dataSource={data?.data || []}
          columns={columns}
          rowKey="id"
          loading={isLoading}
          scroll={{ x: 900 }}
          pagination={{
            current: page,
            pageSize,
            total: data?.total || 0,
            onChange: (p, ps) => {
              setPage(p);
              setPageSize(ps);
            },
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条`,
          }}
        />
      </Card>

      <Modal
        title={editingStudent ? '编辑学生' : '新增学生'}
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => {
          setModalVisible(false);
          setEditingStudent(null);
        }}
        confirmLoading={createMutation.isPending || updateMutation.isPending}
        width={700}
      >
        <Form form={form} layout="vertical" initialValues={{ status: '正常', is_focus: false }}>
          <Row gutter={16}>
            <Col span={8}>
              <Form.Item name="name" label="姓名" rules={[{ required: true, message: '请输入姓名' }]}>
                <Input />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="student_no" label="学号" rules={[{ required: true, message: '请输入学号' }]}>
                <Input />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="gender" label="性别">
                <Select allowClear options={[{ value: '男', label: '男' }, { value: '女', label: '女' }]} />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="phone" label="联系电话">
                <Input />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="parent_name" label="家长姓名">
                <Input />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="parent_phone" label="家长电话">
                <Input />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="group_name" label="所属小组">
                <Input />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="address" label="家庭住址">
            <Input.TextArea rows={2} />
          </Form.Item>
          <Row gutter={16}>
            <Col span={8}>
              <Form.Item name="status" label="状态">
                <Select
                  options={[
                    { value: '正常', label: '正常' },
                    { value: '休学', label: '休学' },
                    { value: '退学', label: '退学' },
                  ]}
                />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="is_focus" label="重点关注">
                <Select
                  options={[
                    { value: false, label: '否' },
                    { value: true, label: '是' },
                  ]}
                />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>

      {/* 导入预览确认弹窗 */}
      <Modal
        title="导入预览"
        open={previewVisible}
        onCancel={() => { setPreviewVisible(false); setPreviewData(null); setImportFilePath(null); }}
        width={900}
        footer={[
          <Button key="cancel" onClick={() => { setPreviewVisible(false); setPreviewData(null); setImportFilePath(null); }}>
            取消
          </Button>,
          <Button
            key="confirm"
            type="primary"
            disabled={!previewData || previewData.valid_rows === 0}
            onClick={handleConfirmImport}
          >
            确认导入 {previewData ? previewData.valid_rows : 0} 条
          </Button>,
        ]}
      >
        {previewLoading ? (
          <Spin style={{ display: 'block', textAlign: 'center', padding: 40 }} />
        ) : previewData ? (
          <>
            <Space style={{ marginBottom: 12 }}>
              <Tag color="blue">共 {previewData.total_rows} 条</Tag>
              <Tag color="green">有效 {previewData.valid_rows} 条</Tag>
              {previewData.error_rows > 0 && <Tag color="red">{previewData.error_rows} 条错误</Tag>}
            </Space>
            {previewData.errors.length > 0 && (
              <Alert
                type="error"
                showIcon
                style={{ marginBottom: 12 }}
                message="以下行存在校验错误，将不会被导入："
                description={
                  <ul style={{ margin: 0, paddingLeft: 16 }}>
                    {previewData.errors.map((e, i) => <li key={i}>{e}</li>)}
                  </ul>
                }
              />
            )}
            <Table
              dataSource={(previewData.rows as Array<Record<string, unknown>>).filter(r => r.valid as boolean)}
              columns={[
                { title: '行号', dataIndex: 'row', key: 'row', width: 60 },
                { title: '学号', dataIndex: 'student_no', key: 'student_no', width: 120,
                  render: (v: string) => <strong>{v}</strong> },
                { title: '姓名', dataIndex: 'name', key: 'name', width: 100 },
                { title: '性别', dataIndex: 'gender', key: 'gender', width: 60,
                  render: (v: string | null) => v || '-' },
                { title: '电话', dataIndex: 'phone', key: 'phone', width: 120,
                  render: (v: string | null) => v || '-' },
                { title: '家长', dataIndex: 'parent_name', key: 'parent_name', width: 80,
                  render: (v: string | null) => v || '-' },
                { title: '家长电话', dataIndex: 'parent_phone', key: 'parent_phone', width: 120,
                  render: (v: string | null) => v || '-' },
                { title: '小组', dataIndex: 'group_name', key: 'group_name', width: 100,
                  render: (v: string | null) => v || '-' },
              ]}
              rowKey="row"
              size="small"
              pagination={false}
              scroll={{ y: 300 }}
            />
          </>
        ) : null}
      </Modal>
    </div>
  );
}

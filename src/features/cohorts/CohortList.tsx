import { useState } from 'react';
import {
  Card, Table, Button, Modal, Form, Input, InputNumber, Select, Space, Tag, message, Popconfirm, Typography, Tooltip,
} from 'antd';
import { PlusOutlined, EditOutlined, SwapOutlined, LockOutlined, UnlockOutlined, ExportOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { cohortService, getCommandErrorMessage } from '@/services';
import type { Cohort, CohortStatus } from '@/types';

const { Title } = Typography;

export default function CohortList() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { isReadonly, setCurrentCohort } = useAppStore();
  const [modalVisible, setModalVisible] = useState(false);
  const [editingCohort, setEditingCohort] = useState<Cohort | null>(null);
  const [form] = Form.useForm();

  const { data: cohorts, isLoading } = useQuery({
    queryKey: ['cohorts'],
    queryFn: () => cohortService.list(),
  });

  // 筛选状态
  const [searchName, setSearchName] = useState('');
  const [filterStatus, setFilterStatus] = useState<string | undefined>();
  const [filterYear, setFilterYear] = useState<number | undefined>();

  const filteredCohorts = (cohorts || []).filter((c) => {
    if (searchName) {
      const kw = searchName.toLowerCase();
      if (!c.cohort_name.toLowerCase().includes(kw) && !c.class_name.toLowerCase().includes(kw)) {
        return false;
      }
    }
    if (filterStatus && c.status !== filterStatus) return false;
    if (filterYear && c.admission_year !== filterYear) return false;
    return true;
  });

  const createMutation = useMutation({
    mutationFn: (data: Partial<Cohort>) => cohortService.create(data),
    onSuccess: async (createdCohort) => {
      if (createdCohort.is_current) {
        setCurrentCohort(createdCohort);
      }
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['cohorts'] }),
        queryClient.invalidateQueries({ queryKey: ['currentCohort'] }),
      ]);
      message.success('届次创建成功');
      setModalVisible(false);
      form.resetFields();
    },
    onError: (err: unknown) => message.error(getCommandErrorMessage(err)),
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<Cohort> }) => cohortService.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['cohorts'] });
      message.success('届次更新成功');
      setModalVisible(false);
      setEditingCohort(null);
    },
    onError: (err: unknown) => message.error(getCommandErrorMessage(err)),
  });

  const archiveMutation = useMutation({
    mutationFn: (id: number) => cohortService.archive(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['cohorts'] });
      queryClient.invalidateQueries({ queryKey: ['currentCohort'] });
      message.success('届次已归档');
    },
    onError: (err: unknown) => message.error(getCommandErrorMessage(err)),
  });

  const unarchiveMutation = useMutation({
    mutationFn: (id: number) => cohortService.unarchive(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['cohorts'] });
      queryClient.invalidateQueries({ queryKey: ['currentCohort'] });
      message.success('届次已解除归档');
    },
    onError: (err: unknown) => message.error(getCommandErrorMessage(err)),
  });


  const handleCreate = () => {
    setEditingCohort(null);
    form.resetFields();
    setModalVisible(true);
  };

  const handleEdit = (cohort: Cohort) => {
    setEditingCohort(cohort);
    form.setFieldsValue(cohort);
    setModalVisible(true);
  };

  const handleSubmit = async () => {
    const values = await form.validateFields();
    if (editingCohort) {
      updateMutation.mutate({ id: editingCohort.id, data: values });
    } else {
      createMutation.mutate(values);
    }
  };

  const handleSwitch = (cohort: Cohort) => {
    Modal.confirm({
      title: '切换届次',
      content: `确定切换到 ${cohort.cohort_name} ${cohort.class_name} 吗？`,
      onOk: async () => {
        try {
          await cohortService.setCurrent(cohort.id);
          setCurrentCohort(cohort);
          message.success(`已切换到 ${cohort.cohort_name} ${cohort.class_name}`);
          // 清除所有依赖于届次的查询缓存，触发页面自然重渲染
          queryClient.invalidateQueries();
        } catch (err: unknown) {
          message.error(getCommandErrorMessage(err));
        }
      },
    });
  };

  const columns = [
    { title: '届次名称', dataIndex: 'cohort_name', key: 'cohort_name' },
    { title: '班级名称', dataIndex: 'class_name', key: 'class_name' },
    { title: '入学年份', dataIndex: 'admission_year', key: 'admission_year' },
    { title: '毕业年份', dataIndex: 'graduation_year', key: 'graduation_year' },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: CohortStatus) =>
        status === '已归档' ? <Tag>已归档</Tag> : <Tag color="green">使用中</Tag>,
    },
    {
      title: '当前',
      key: 'is_current',
      render: (_: unknown, record: Cohort) =>
        record.is_current ? <Tag color="blue">当前</Tag> : null,
    },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Cohort) => (
        <Space>
          {!record.is_current && !isReadonly && (
            <Tooltip title="切换为此届次">
              <Button type="link" icon={<SwapOutlined />} onClick={() => handleSwitch(record)}>
                切换
              </Button>
            </Tooltip>
          )}
          <Button type="link" icon={<EditOutlined />} onClick={() => handleEdit(record)}>
            {isReadonly ? '查看' : '编辑'}
          </Button>
          {record.status === '使用中' && (
            <Popconfirm title="归档后数据将变为只读，确定归档？" onConfirm={() => archiveMutation.mutate(record.id)}>
              <Button type="link" icon={<LockOutlined />} danger>
                归档
              </Button>
            </Popconfirm>
          )}
          {record.status === '已归档' && (
            <Popconfirm title="解除归档将恢复可编辑状态，确定解除？" onConfirm={() => unarchiveMutation.mutate(record.id)}>
              <Button type="link" icon={<UnlockOutlined />}>
                解除归档
              </Button>
            </Popconfirm>
          )}
          <Button type="link" icon={<ExportOutlined />} onClick={() => navigate(`/settings?export=${record.id}`)}>
            导出
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div className="page-header">
        <Title level={4}>届次管理</Title>
        <Button type="primary" icon={<PlusOutlined />} onClick={handleCreate}>
          新建届次
        </Button>
      </div>

      <Card size="small" style={{ marginBottom: 16 }}>
        <Space wrap>
          <Input
            placeholder="搜索届次/班级名称"
            allowClear
            style={{ width: 200 }}
            value={searchName}
            onChange={(e) => setSearchName(e.target.value)}
          />
          <Select
            placeholder="状态筛选"
            allowClear
            style={{ width: 140 }}
            value={filterStatus}
            onChange={setFilterStatus}
            options={[
              { value: '使用中', label: '使用中' },
              { value: '已归档', label: '已归档' },
            ]}
          />
          <Input
            placeholder="入学年份"
            type="number"
            style={{ width: 120 }}
            value={filterYear || ''}
            onChange={(e) => setFilterYear(e.target.value ? Number(e.target.value) : undefined)}
          />
        </Space>
      </Card>

      <Card>
        <Table
          dataSource={filteredCohorts}
          columns={columns}
          rowKey="id"
          loading={isLoading}
          pagination={false}
        />
      </Card>

      <Modal
        title={editingCohort ? '编辑届次' : '新建届次'}
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => {
          setModalVisible(false);
          setEditingCohort(null);
        }}
        confirmLoading={createMutation.isPending || updateMutation.isPending}
        width={600}
      >
        <Form form={form} layout="vertical" initialValues={{ status: '使用中' }}>
          <Space size="large" wrap>
            <Form.Item name="cohort_name" label="届次名称" rules={[{ required: true, message: '请输入届次名称' }]}>
              <Input placeholder="如：2025届" style={{ width: 200 }} />
            </Form.Item>
            <Form.Item name="class_name" label="班级名称" rules={[{ required: true, message: '请输入班级名称' }]}>
              <Input placeholder="如：1班" style={{ width: 200 }} />
            </Form.Item>
          </Space>
          <Space size="large" wrap>
            <Form.Item name="school_name" label="学校名称">
              <Input style={{ width: 250 }} />
            </Form.Item>
            <Form.Item name="head_teacher" label="班主任">
              <Input style={{ width: 200 }} />
            </Form.Item>
          </Space>
          <Space size="large" wrap>
            <Form.Item name="admission_year" label="入学年份">
              <InputNumber min={2000} max={2099} />
            </Form.Item>
            <Form.Item name="graduation_year" label="毕业年份">
              <InputNumber min={2000} max={2099} />
            </Form.Item>
            <Form.Item name="semester" label="当前学期">
              <Select style={{ width: 120 }}>
                <Select.Option value="第一学期">第一学期</Select.Option>
                <Select.Option value="第二学期">第二学期</Select.Option>
              </Select>
            </Form.Item>
          </Space>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

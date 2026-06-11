import { useState } from 'react';
import { Card, Table, Button, Modal, Form, Input, DatePicker, Space, Tag, message, Popconfirm, Typography, Progress, Row, Col } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined, EyeOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { homeworkService } from '@/services';
import type { Homework } from '@/types';
import dayjs from 'dayjs';

const { Title } = Typography;

export default function HomeworkList() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [modalVisible, setModalVisible] = useState(false);
  const [editingHomework, setEditingHomework] = useState<Homework | null>(null);
  const [searchText, setSearchText] = useState('');
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const [form] = Form.useForm();

  const { data, isLoading } = useQuery({
    queryKey: ['homeworks', currentCohort?.id, page, pageSize, searchText],
    queryFn: () => homeworkService.list(currentCohort!.id, { page, page_size: pageSize, search: searchText || undefined }),
    enabled: !!currentCohort,
  });

  const createMutation = useMutation({
    mutationFn: (data: Partial<Homework>) => homeworkService.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['homeworks'] });
      message.success('作业创建成功');
      setModalVisible(false);
      form.resetFields();
    },
    onError: (err: Error) => message.error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<Homework> }) => homeworkService.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['homeworks'] });
      message.success('作业更新成功');
      setModalVisible(false);
      setEditingHomework(null);
    },
    onError: (err: Error) => message.error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => homeworkService.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['homeworks'] });
      message.success('作业已删除');
    },
    onError: (err: Error) => message.error(err.message),
  });

  const handleCreate = () => {
    if (isReadonly) return;
    setEditingHomework(null);
    form.resetFields();
    setModalVisible(true);
  };

  const handleEdit = (homework: Homework) => {
    if (isReadonly) return;
    setEditingHomework(homework);
    form.setFieldsValue({
      ...homework,
      publish_date: homework.publish_date ? dayjs(homework.publish_date) : undefined,
      deadline: homework.deadline ? dayjs(homework.deadline) : undefined,
    });
    setModalVisible(true);
  };

  const handleSubmit = async () => {
    const values = await form.validateFields();
    values.cohort_id = currentCohort!.id;
    values.publish_date = values.publish_date?.format('YYYY-MM-DD');
    values.deadline = values.deadline?.format('YYYY-MM-DD');
    if (editingHomework) {
      updateMutation.mutate({ id: editingHomework.id, data: values });
    } else {
      createMutation.mutate(values);
    }
  };

  const columns = [
    { title: '标题', dataIndex: 'title', key: 'title' },
    { title: '科目', dataIndex: 'subject_name', key: 'subject_name', render: (v: string | null) => v || '-' },
    { title: '发布日期', dataIndex: 'publish_date', key: 'publish_date' },
    { title: '截止日期', dataIndex: 'deadline', key: 'deadline', render: (v: string | null) => v || '-' },
    {
      title: '完成率',
      key: 'completion_rate',
      render: (_: unknown, record: Homework) => {
        const rate = record.completion_rate || 0;
        return <Progress percent={Math.round(rate * 100)} size="small" />;
      },
    },
    {
      title: '未完成',
      dataIndex: 'incomplete_count',
      key: 'incomplete_count',
      render: (v: number | undefined) => (v ? <Tag color="red">{v} 人</Tag> : '-'),
    },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Homework) => (
        <Space>
          <Button type="link" icon={<EyeOutlined />} onClick={() => navigate(`/homework/${record.id}`)}>
            登记
          </Button>
          <Button type="link" icon={<EditOutlined />} disabled={isReadonly} onClick={() => handleEdit(record)}>
            编辑
          </Button>
          <Popconfirm title="确定删除该作业？" onConfirm={() => deleteMutation.mutate(record.id)}>
            <Button type="link" icon={<DeleteOutlined />} disabled={isReadonly} danger>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <div className="page-header">
        <Title level={4}>作业管理</Title>
        {!isReadonly && (
          <Button type="primary" icon={<PlusOutlined />} onClick={handleCreate}>
            创建作业
          </Button>
        )}
      </div>

      <Card>
        <div className="filter-bar">
          <Input.Search placeholder="搜索作业标题" allowClear onSearch={setSearchText} style={{ width: 250 }} />
        </div>

        <Table
          dataSource={data?.data || []}
          columns={columns}
          rowKey="id"
          loading={isLoading}
          pagination={{
            current: page,
            pageSize,
            total: data?.total || 0,
            onChange: (p, ps) => { setPage(p); setPageSize(ps); },
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条`,
          }}
        />
      </Card>

      <Modal
        title={editingHomework ? '编辑作业' : '创建作业'}
        open={modalVisible}
        onOk={handleSubmit}
        onCancel={() => { setModalVisible(false); setEditingHomework(null); }}
        confirmLoading={createMutation.isPending || updateMutation.isPending}
        width={600}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="title" label="作业标题" rules={[{ required: true, message: '请输入作业标题' }]}>
            <Input />
          </Form.Item>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="subject_name" label="科目">
                <Input placeholder="如：数学" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="subject_id" label="科目ID（可选）">
                <Input type="number" />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="publish_date" label="发布日期">
                <DatePicker style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="deadline" label="截止日期">
                <DatePicker style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="description" label="作业描述">
            <Input.TextArea rows={3} />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

import { useEffect, useState } from 'react';
import { Card, Table, Button, Modal, Form, Input, DatePicker, Space, Tag, message, Popconfirm, Typography, Progress, Row, Col, Select, Switch } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined, EyeOutlined, LinkOutlined, UploadOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { homeworkService, studentService } from '@/services';
import { useLocalStorageState } from '@/hooks/useLocalStorageState';
import { useKeyboardShortcut } from '@/hooks/useKeyboardShortcut';
import type { Homework, Student } from '@/types';
import dayjs from 'dayjs';

const { Title } = Typography;

export default function HomeworkList() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [modalVisible, setModalVisible] = useState(false);
  const [editingHomework, setEditingHomework] = useState<Homework | null>(null);
  const [searchText, setSearchText] = useLocalStorageState('homework_list_search', '');
  const [publishDateFilter, setPublishDateFilter] = useLocalStorageState<string | undefined>('homework_list_publish_date', undefined);
  const [incompleteOnly, setIncompleteOnly] = useLocalStorageState('homework_list_incomplete_only', false);
  const [page, setPage] = useLocalStorageState('homework_list_page', 1);
  const [pageSize, setPageSize] = useLocalStorageState('homework_list_page_size', 10);
  const [form] = Form.useForm();

  const { data: allStudents } = useQuery({
    queryKey: ['allStudents', currentCohort?.id],
    queryFn: () => studentService.getAll(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data, isLoading } = useQuery({
    queryKey: ['homeworks', currentCohort?.id, page, pageSize, searchText, publishDateFilter, incompleteOnly],
    queryFn: () =>
      homeworkService.list(currentCohort!.id, {
        page,
        page_size: pageSize,
        search: searchText || undefined,
        publish_date: publishDateFilter,
        incomplete_only: incompleteOnly || undefined,
      }),
    enabled: !!currentCohort,
  });

  useEffect(() => {
    const publishDate = searchParams.get('publish_date');
    const pending = searchParams.get('incomplete_only');
    if (publishDate) {
      setPublishDateFilter(publishDate);
    }
    if (pending === '1') {
      setIncompleteOnly(true);
    }
    if (publishDate || pending) {
      setPage(1);
    }
  }, [searchParams, setIncompleteOnly, setPage, setPublishDateFilter]);

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
    form.setFieldValue('assigned_student_ids', (allStudents || []).filter((student) => student.status === '正常').map((student) => student.id));
    setModalVisible(true);
  };

  const handleEdit = (homework: Homework) => {
    if (isReadonly) return;
    setEditingHomework(homework);
    form.setFieldsValue({
      ...homework,
      attachment_source_path: undefined,
      clear_attachment: false,
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

  const handleSelectAttachment = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const filePath = await open({ multiple: false });
      if (filePath) {
        form.setFieldValue('attachment_source_path', filePath);
        form.setFieldValue('clear_attachment', false);
      }
    } catch {
      message.error('选择附件失败');
    }
  };

  const assignableStudents = (allStudents || []).filter((student) => student.status === '正常');

  const columns = [
    { title: '标题', dataIndex: 'title', key: 'title', width: 180 },
    { title: '科目', dataIndex: 'subject_name', key: 'subject_name', width: 100, render: (v: string | null) => v || '-' },
    { title: '发布日期', dataIndex: 'publish_date', key: 'publish_date', width: 120 },
    { title: '截止日期', dataIndex: 'deadline', key: 'deadline', width: 120, render: (v: string | null) => v || '-' },
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
      title: '附件',
      key: 'attachment',
      render: (_: unknown, record: Homework) =>
        record.attachment_name ? (
          <Button
            type="link"
            icon={<LinkOutlined />}
            onClick={async () => {
              try {
                await homeworkService.openAttachment(record.id);
              } catch (error) {
                message.error(error instanceof Error ? error.message : '打开附件失败');
              }
            }}
          >
            {record.attachment_name}
          </Button>
        ) : (
          '-'
        ),
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
  useKeyboardShortcut('n', handleCreate, { ctrlOrMeta: true, enabled: !isReadonly });

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
          <Input.Search
            placeholder="搜索作业标题"
            allowClear
            value={searchText}
            onChange={(event) => setSearchText(event.target.value)}
            onSearch={(value) => {
              setSearchText(value);
              setPage(1);
            }}
            style={{ width: 250 }}
          />
          <DatePicker
            value={publishDateFilter ? dayjs(publishDateFilter) : null}
            onChange={(value) => {
              setPublishDateFilter(value ? value.format('YYYY-MM-DD') : undefined);
              setPage(1);
            }}
            placeholder="发布日期"
            allowClear
          />
          <Space>
            <span>仅看待处理</span>
            <Switch
              checked={incompleteOnly}
              onChange={(checked) => {
                setIncompleteOnly(checked);
                setPage(1);
              }}
            />
          </Space>
        </div>

        <Table
          dataSource={data?.data || []}
          columns={columns}
          rowKey="id"
          loading={isLoading}
          scroll={{ x: 980 }}
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
            <Col span={24}>
              <Form.Item name="subject_name" label="科目">
                <Input placeholder="如：数学" />
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
          {!editingHomework && (
            <Form.Item
              name="assigned_student_ids"
              label="分配学生"
              rules={[{ required: true, message: '请至少选择一名学生' }]}
              extra="默认已选当前届次全部正常学生，你可以手动删减。"
            >
              <Select
                mode="multiple"
                allowClear
                showSearch
                optionFilterProp="label"
                maxTagCount={3}
                maxTagPlaceholder={(omittedValues) => `等 ${omittedValues.length} 人`}
                placeholder="选择要布置这份作业的学生"
                options={assignableStudents.map((student: Student) => ({
                  value: student.id,
                  label: `${student.student_no} ${student.name}${student.group_name ? ` · ${student.group_name}` : ''}`,
                }))}
              />
            </Form.Item>
          )}
          <Form.Item name="description" label="作业描述">
            <Input.TextArea rows={3} />
          </Form.Item>
          <Form.Item label="作业附件">
            <Space direction="vertical" style={{ width: '100%' }}>
              <Input
                value={form.getFieldValue('attachment_source_path') || editingHomework?.attachment_name || ''}
                readOnly
                placeholder="可上传讲义、题单等附件"
              />
              <Space>
                <Button icon={<UploadOutlined />} onClick={handleSelectAttachment}>
                  选择附件
                </Button>
                {editingHomework?.attachment_name && (
                  <>
                    <Button
                      icon={<LinkOutlined />}
                      onClick={async () => {
                        try {
                          await homeworkService.openAttachment(editingHomework.id);
                        } catch (error) {
                          message.error(error instanceof Error ? error.message : '打开附件失败');
                        }
                      }}
                    >
                      打开当前附件
                    </Button>
                    <Button
                      danger
                      onClick={() => {
                        form.setFieldValue('attachment_source_path', undefined);
                        form.setFieldValue('clear_attachment', true);
                      }}
                    >
                      删除当前附件
                    </Button>
                  </>
                )}
              </Space>
              <Form.Item name="attachment_source_path" hidden>
                <Input />
              </Form.Item>
              <Form.Item name="clear_attachment" hidden>
                <Input />
              </Form.Item>
            </Space>
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}

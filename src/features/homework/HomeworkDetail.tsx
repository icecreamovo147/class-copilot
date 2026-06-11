import { Card, Table, Button, Select, Tag, message, Typography, Space, Spin, Alert, Empty, Descriptions } from 'antd';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { homeworkService } from '@/services';
import type { HomeworkRecord, HomeworkStatus } from '@/types';
import { HOMEWORK_STATUSES } from '@/types';

const { Title } = Typography;

const statusColors: Record<HomeworkStatus, string> = {
  '未登记': 'default',
  '已完成': 'green',
  '未完成': 'red',
  '迟交': 'orange',
  '补交': 'blue',
  '质量较差': 'purple',
};

export default function HomeworkDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { isReadonly } = useAppStore();

  const { data: homework } = useQuery({
    queryKey: ['homework', Number(id)],
    queryFn: () => homeworkService.getById(Number(id)),
    enabled: !!id,
  });

  const { data: records, isLoading, error } = useQuery({
    queryKey: ['homeworkRecords', Number(id)],
    queryFn: () => homeworkService.getRecords(Number(id)),
    enabled: !!id,
  });

  const updateMutation = useMutation({
    mutationFn: ({ recordId, status, remark }: { recordId: number; status: string; remark?: string }) =>
      homeworkService.updateRecord(recordId, status, remark),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['homeworkRecords', Number(id)] });
      message.success('状态已更新');
    },
    onError: (err: Error) => message.error(err.message),
  });

  const batchMutation = useMutation({
    mutationFn: ({ studentIds, status }: { studentIds: number[]; status: string }) =>
      homeworkService.batchUpdateRecords(Number(id), studentIds, status),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['homeworkRecords', Number(id)] });
      message.success('批量更新成功');
    },
    onError: (err: Error) => message.error(err.message),
  });

  const handleBatchComplete = () => {
    const allIds = (records || []).map((r) => r.student_id);
    batchMutation.mutate({ studentIds: allIds, status: '已完成' });
  };

  const handleBatchIncomplete = () => {
    const unregisteredIds = (records || []).filter((r) => r.status === '未登记').map((r) => r.student_id);
    if (unregisteredIds.length === 0) {
      message.info('没有未登记的学生');
      return;
    }
    batchMutation.mutate({ studentIds: unregisteredIds, status: '未完成' });
  };

  const handleExportIncomplete = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: `未交名单_${homework?.title}.xlsx`,
      });
      if (filePath) {
        await homeworkService.exportIncomplete(Number(id), filePath);
        message.success('导出成功');
      }
    } catch {
      message.error('导出失败');
    }
  };

  if (isLoading) return <Spin size="large" style={{ display: 'block', textAlign: 'center', marginTop: 100 }} />;
  if (error) return <Alert message="加载作业数据失败" type="error" showIcon />;
  if (!homework || !records) return <Empty description="未找到作业" />;

  const completed = records.filter((r) => r.status === '已完成').length;
  const total = records.length;
  const rate = total > 0 ? (completed / total) : 0;
  const incompleteCount = records.filter((r) => r.status === '未完成' || r.status === '未登记').length;

  const columns = [
    { title: '姓名', dataIndex: 'student_name', key: 'student_name' },
    { title: '学号', dataIndex: 'student_no', key: 'student_no' },
    { title: '小组', dataIndex: 'group_name', key: 'group_name' },
    {
      title: '完成状态',
      dataIndex: 'status',
      key: 'status',
      render: (status: HomeworkStatus, record: HomeworkRecord) => (
        <Select
          value={status}
          style={{ width: 120 }}
          disabled={isReadonly}
          onChange={(val) => updateMutation.mutate({ recordId: record.id, status: val })}
          options={HOMEWORK_STATUSES.map((s) => ({
            value: s,
            label: <Tag color={statusColors[s]}>{s}</Tag>,
          }))}
        />
      ),
    },
    { title: '备注', dataIndex: 'remark', key: 'remark', ellipsis: true },
  ];

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <Title level={4}>作业完成情况登记</Title>
        <Button onClick={() => navigate('/homework')}>返回列表</Button>
      </div>

      <Card size="small" style={{ marginBottom: 16 }}>
        <Descriptions column={4} size="small">
          <Descriptions.Item label="标题">{homework.title}</Descriptions.Item>
          <Descriptions.Item label="科目">{homework.subject_name || '-'}</Descriptions.Item>
          <Descriptions.Item label="发布日期">{homework.publish_date}</Descriptions.Item>
          <Descriptions.Item label="截止日期">{homework.deadline || '-'}</Descriptions.Item>
        </Descriptions>
        <Descriptions column={4} size="small">
          <Descriptions.Item label="总人数">{total}</Descriptions.Item>
          <Descriptions.Item label="已完成">{completed}</Descriptions.Item>
          <Descriptions.Item label="完成率">{(rate * 100).toFixed(1)}%</Descriptions.Item>
          <Descriptions.Item label="未完成">
            {incompleteCount > 0 ? <Tag color="red">{incompleteCount} 人</Tag> : '无'}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Space style={{ marginBottom: 16 }}>
        {!isReadonly && (
          <>
            <Button type="primary" onClick={handleBatchComplete} loading={batchMutation.isPending}>
              全部已完成
            </Button>
            <Button onClick={handleBatchIncomplete} loading={batchMutation.isPending}>
              批量未完成（仅未登记）
            </Button>
          </>
        )}
        <Button onClick={handleExportIncomplete}>导出未交名单</Button>
      </Space>

      <Card>
        <Table
          dataSource={records}
          columns={columns}
          rowKey="id"
          size="small"
          pagination={false}
        />
      </Card>
    </div>
  );
}

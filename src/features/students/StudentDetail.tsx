
import { Card, Descriptions, Tabs, Table, Tag, Typography, Spin, Alert, Empty } from 'antd';
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { statisticsService } from '@/services';

const { Title } = Typography;

export default function StudentDetail() {
  const { id } = useParams<{ id: string }>();

  const { data: profile, isLoading, error } = useQuery({
    queryKey: ['studentProfile', Number(id)],
    queryFn: () => statisticsService.studentProfile(Number(id)),
    enabled: !!id,
  });

  if (isLoading) return <Spin size="large" style={{ display: 'block', textAlign: 'center', marginTop: 100 }} />;
  if (error) return <Alert message="加载学生信息失败" type="error" showIcon />;
  if (!profile) return <Empty description="未找到学生信息" />;

  const { student } = profile;

  const tabItems = [
    {
      key: 'homework',
      label: '作业记录',
      children: (
        <div>
          <Descriptions column={3} bordered size="small" style={{ marginBottom: 16 }}>
            <Descriptions.Item label="总作业数">{profile.homework.total}</Descriptions.Item>
            <Descriptions.Item label="已完成">{profile.homework.completed}</Descriptions.Item>
            <Descriptions.Item label="完成率">{(profile.homework.rate * 100).toFixed(1)}%</Descriptions.Item>
          </Descriptions>
          {profile.homework.consecutive_incomplete > 0 && (
            <Tag color="red" style={{ marginBottom: 8 }}>连续未交 {profile.homework.consecutive_incomplete} 次</Tag>
          )}
        </div>
      ),
    },
    {
      key: 'attendance',
      label: '考勤记录',
      children: (
        <Descriptions column={4} bordered size="small">
          <Descriptions.Item label="总天数">{profile.attendance.total}</Descriptions.Item>
          <Descriptions.Item label="正常">{profile.attendance.normal}</Descriptions.Item>
          <Descriptions.Item label="异常">{profile.attendance.abnormal}</Descriptions.Item>
          <Descriptions.Item label="出勤率">{(profile.attendance.rate * 100).toFixed(1)}%</Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'scores',
      label: '成绩记录',
      children: (
        <Table
          dataSource={profile.scores}
          columns={[
            { title: '考试', dataIndex: 'exam_name', key: 'exam_name' },
            { title: '科目', dataIndex: 'subject_name', key: 'subject_name' },
            { title: '成绩', dataIndex: 'score_value', key: 'score_value', render: (v: number | null) => v ?? '-' },
          ]}
          rowKey={(_, index) => String(index)}
          pagination={false}
          size="small"
        />
      ),
    },
    {
      key: 'behaviors',
      label: '奖惩记录',
      children: (
        <Table
          dataSource={profile.behaviors}
          columns={[
            { title: '日期', dataIndex: 'record_date', key: 'record_date' },
            { title: '类型', dataIndex: 'type', key: 'type', render: (t: string) => <Tag>{t}</Tag> },
            { title: '标题', dataIndex: 'title', key: 'title' },
            { title: '分值', dataIndex: 'score', key: 'score', render: (v: number) => v !== 0 ? v : '-' },
          ]}
          rowKey="id"
          pagination={false}
          size="small"
        />
      ),
    },
  ];

  return (
    <div>
      <Title level={4}>学生详情</Title>
      <Card style={{ marginBottom: 16 }}>
        <Descriptions title="基础信息" bordered size="small" column={3}>
          <Descriptions.Item label="姓名">{student.name}</Descriptions.Item>
          <Descriptions.Item label="学号">{student.student_no}</Descriptions.Item>
          <Descriptions.Item label="性别">{student.gender || '-'}</Descriptions.Item>
          <Descriptions.Item label="联系电话">{student.phone || '-'}</Descriptions.Item>
          <Descriptions.Item label="家长姓名">{student.parent_name || '-'}</Descriptions.Item>
          <Descriptions.Item label="家长电话">{student.parent_phone || '-'}</Descriptions.Item>
          <Descriptions.Item label="所属小组">{student.group_name || '-'}</Descriptions.Item>
          <Descriptions.Item label="状态">
            <Tag color={student.status === '正常' ? 'green' : 'orange'}>{student.status}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="重点关注">
            {student.is_focus ? <Tag color="red">是</Tag> : '否'}
          </Descriptions.Item>
          <Descriptions.Item label="备注">{student.remark || '-'}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card>
        <Tabs items={tabItems} />
      </Card>
    </div>
  );
}

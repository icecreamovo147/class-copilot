import { Card, Descriptions, Tabs, Table, Tag, Typography, Spin, Alert, Empty, Button, Space, message } from 'antd';
import { DownloadOutlined } from '@ant-design/icons';
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { statisticsService, homeworkService, attendanceService } from '@/services';
const { Title } = Typography;

const statusColors: Record<string, string> = {
  '正常': 'green', '迟到': 'orange', '早退': 'gold', '请假': 'blue', '旷课': 'red',
};

const hwStatusColors: Record<string, string> = {
  '已完成': 'green', '未完成': 'red', '未登记': 'default', '迟交': 'orange', '补交': 'gold', '质量较差': 'magenta',
};

export default function StudentDetail() {
  const { id } = useParams<{ id: string }>();
  const studentId = Number(id);

  const { data: profile, isLoading, error } = useQuery({
    queryKey: ['studentProfile', studentId],
    queryFn: () => statisticsService.studentProfile(studentId),
    enabled: !!id,
  });

  // 单独获取作业记录明细（明细页才需展示）
  const { data: homeworkRecords } = useQuery({
    queryKey: ['studentHomeworkRecords', studentId],
    queryFn: () => homeworkService.getStudentRecords(studentId),
    enabled: !!id,
  });

  // 单独获取考勤记录明细
  const { data: attendanceRecords } = useQuery({
    queryKey: ['studentAttendanceRecords', studentId],
    queryFn: async () => {
      const result = await attendanceService.query(
        profile!.student.cohort_id,
        { student_id: studentId, page: 1, page_size: 1000 }
      );
      return result.data;
    },
    enabled: !!profile,
  });

  if (isLoading) return <Spin size="large" style={{ display: 'block', textAlign: 'center', marginTop: 100 }} />;
  if (error) return <Alert message="加载学生信息失败" type="error" showIcon />;
  if (!profile) return <Empty description="未找到学生信息" />;

  const { student } = profile;

  const handleExportGrowthArchive = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: `${student.name}_成长档案.xlsx`,
      });
      if (filePath) {
        await statisticsService.exportStudentGrowthArchive(studentId, filePath);
        message.success('成长档案导出成功');
      }
    } catch {
      message.error('成长档案导出失败');
    }
  };

  const handleExportGrowthArchivePdf = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
        defaultPath: `${student.name}_成长档案.pdf`,
      });
      if (filePath) {
        await statisticsService.exportStudentGrowthArchivePdf(studentId, filePath);
        message.success('成长档案 PDF 导出成功');
      }
    } catch {
      message.error('成长档案 PDF 导出失败');
    }
  };

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
          <Table
            dataSource={homeworkRecords || []}
            columns={[
              { title: '作业标题', dataIndex: 'homework_title', key: 'homework_title' },
              { title: '发布日期', dataIndex: 'publish_date', key: 'publish_date', width: 120 },
              { title: '科目', dataIndex: 'subject_name', key: 'subject_name', width: 100, render: (v: string | null) => v || '-' },
              {
                title: '状态',
                dataIndex: 'status',
                key: 'status',
                width: 100,
                render: (s: string) => <Tag color={hwStatusColors[s] || 'default'}>{s}</Tag>,
              },
              { title: '提交时间', dataIndex: 'submit_time', key: 'submit_time', width: 140, render: (v: string | null) => v || '-' },
            ]}
            rowKey="id"
            size="small"
            pagination={false}
          />
        </div>
      ),
    },
    {
      key: 'attendance',
      label: '考勤记录',
      children: (
        <div>
          <Descriptions column={4} bordered size="small" style={{ marginBottom: 16 }}>
            <Descriptions.Item label="总天数">{profile.attendance.total}</Descriptions.Item>
            <Descriptions.Item label="正常">{profile.attendance.normal}</Descriptions.Item>
            <Descriptions.Item label="异常">{profile.attendance.abnormal}</Descriptions.Item>
            <Descriptions.Item label="出勤率">{(profile.attendance.rate * 100).toFixed(1)}%</Descriptions.Item>
          </Descriptions>
          <Table
            dataSource={attendanceRecords || []}
            columns={[
              { title: '日期', dataIndex: 'attendance_date', key: 'attendance_date', width: 120 },
              {
                title: '状态',
                dataIndex: 'status',
                key: 'status',
                width: 80,
                render: (s: string) => <Tag color={statusColors[s] || 'default'}>{s}</Tag>,
              },
              { title: '原因', dataIndex: 'reason', key: 'reason', ellipsis: true },
              { title: '备注', dataIndex: 'remark', key: 'remark', ellipsis: true },
            ]}
            rowKey="id"
            size="small"
            pagination={false}
          />
        </div>
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
              { title: '日期', dataIndex: 'exam_point', key: 'exam_point', width: 120 },
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
      <Space style={{ marginBottom: 16 }}>
        <Button icon={<DownloadOutlined />} onClick={handleExportGrowthArchive}>
          导出成长档案 Excel
        </Button>
        <Button icon={<DownloadOutlined />} onClick={handleExportGrowthArchivePdf}>
          导出成长档案 PDF
        </Button>
      </Space>
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
          <Descriptions.Item label="关注原因" span={3}>
            {profile.focus_reasons.length > 0 ? profile.focus_reasons.join('；') : '无'}
          </Descriptions.Item>
          <Descriptions.Item label="综合评价" span={3}>
            {profile.overall_evaluation}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card>
        <Tabs items={tabItems} />
      </Card>
    </div>
  );
}

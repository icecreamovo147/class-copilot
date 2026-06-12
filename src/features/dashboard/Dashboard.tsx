import { lazy, Suspense } from 'react';
import { Card, Row, Col, Statistic, Button, Table, Tag, List, Typography, Empty, Spin, Alert, Space } from 'antd';
import {
  UserOutlined,
  BookOutlined,
  CalendarOutlined,
  WarningOutlined,
  PlusOutlined,
  EditOutlined,
  FileExcelOutlined,
} from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { statisticsService } from '@/services';
import dayjs from 'dayjs';

const LazyLine = lazy(() => import('@ant-design/charts').then((m) => ({ default: m.Line })));
const ChartFallback = () => <div style={{ height: 200, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Spin /></div>;

const { Title, Text } = Typography;

export default function Dashboard() {
  const navigate = useNavigate();
  const { currentCohort, isReadonly } = useAppStore();
  const today = dayjs().format('YYYY-MM-DD');

  const goToStudents = (focusOnly = false) => navigate(focusOnly ? '/students?focus=1' : '/students');
  const goToHomeworkToday = () => navigate(`/homework?publish_date=${today}`);
  const goToHomeworkPending = () => navigate('/homework?incomplete_only=1');
  const goToAttendanceToday = () => navigate(`/attendance?tab=query&start_date=${today}&end_date=${today}`);
  const goToAttendanceRegister = () => navigate(`/attendance?tab=register&date=${today}`);

  const { data: stats, isLoading, error } = useQuery({
    queryKey: ['dashboard', currentCohort?.id],
    queryFn: () => statisticsService.dashboard(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data: homeworkTrend = [] } = useQuery({
    queryKey: ['dashboard', 'homeworkTrend', currentCohort?.id],
    queryFn: () => statisticsService.homeworkTrend(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data: attendanceTrend = [] } = useQuery({
    queryKey: ['dashboard', 'attendanceTrend', currentCohort?.id],
    queryFn: () => statisticsService.attendanceTrend(currentCohort!.id, '2000-01-01', '2099-12-31'),
    enabled: !!currentCohort,
  });

  const { data: scoreTrend = [] } = useQuery({
    queryKey: ['dashboard', 'scoreTrend', currentCohort?.id],
    queryFn: () => statisticsService.scoreTrend(currentCohort!.id),
    enabled: !!currentCohort,
  });

  if (!currentCohort) {
    return (
      <Empty description="请先创建或选择一个当前届次">
        <Button type="primary" onClick={() => navigate('/cohorts')}>
          前往届次管理
        </Button>
      </Empty>
    );
  }

  if (isLoading) {
    return <Spin size="large" style={{ display: 'block', textAlign: 'center', marginTop: 100 }} />;
  }

  if (error) {
    return <Alert message="加载看板数据失败" type="error" showIcon />;
  }

  if (!stats) {
    return <Empty description="暂无数据" />;
  }

  return (
    <div>
      <div className="page-header">
        <Title level={4}>
          {currentCohort.cohort_name} {currentCohort.class_name}
          {isReadonly && <Tag style={{ marginLeft: 8 }}>已归档</Tag>}
        </Title>
        {!isReadonly && (
          <Space>
            <Button type="primary" icon={<PlusOutlined />} onClick={() => navigate('/students')}>
              新增学生
            </Button>
            <Button icon={<EditOutlined />} onClick={() => navigate('/homework')}>
              创建作业
            </Button>
            <Button icon={<CalendarOutlined />} onClick={() => navigate('/attendance')}>
              考勤登记
            </Button>
            <Button icon={<FileExcelOutlined />} onClick={() => navigate('/scores')}>
              导入成绩
            </Button>
          </Space>
        )}
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} md={6}>
          <Card hoverable onClick={() => goToStudents(false)}>
            <Statistic title="班级人数" value={stats.total_students} prefix={<UserOutlined />} suffix={`人`} />
            <Text type="secondary">男 {stats.male_count} / 女 {stats.female_count}</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card hoverable onClick={goToHomeworkToday}>
            <Statistic
              title="今日作业完成率"
              value={stats.today_homework_count > 0 ? Number((stats.today_homework_rate * 100).toFixed(1)) : 0}
              suffix="%"
              precision={1}
              prefix={<BookOutlined />}
              valueStyle={{
                color: stats.today_homework_count === 0
                  ? undefined
                  : stats.today_homework_rate >= 0.8
                    ? '#3f8600'
                    : '#cf1322',
              }}
            />
            <Text type="secondary">共{stats.today_homework_count}项 / {stats.today_homework_completed}/{stats.today_homework_total_records} 已完成</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card hoverable onClick={goToAttendanceToday}>
            <Statistic title="今日考勤" value={stats.today_attendance_normal} prefix={<CalendarOutlined />} suffix={`人正常`} />
            <Text type="secondary">
              迟到{stats.today_attendance_late} 请假{stats.today_attendance_leave} 旷课{stats.today_attendance_absent}
            </Text>
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card hoverable onClick={stats.pending_attendance ? goToAttendanceRegister : goToHomeworkPending}>
            <Statistic title="待处理事项" value={stats.pending_homework + (stats.pending_attendance ? 1 : 0)} prefix={<WarningOutlined />} />
            <div>
              {stats.pending_homework > 0 && (
                <Button type="link" style={{ padding: 0, height: 'auto' }} onClick={(event) => { event.stopPropagation(); goToHomeworkPending(); }}>
                  未登记作业：{stats.pending_homework} 项
                </Button>
              )}
              {stats.pending_attendance && (
                <Button type="link" style={{ padding: 0, height: 'auto', display: 'block' }} onClick={(event) => { event.stopPropagation(); goToAttendanceRegister(); }}>
                  今日考勤未登记
                </Button>
              )}
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={12}>
          <Card title="今日作业情况" size="small">
            {stats.today_homework_count > 0 ? (
              <Table
                dataSource={[
                  { key: '1', label: '已布置作业', value: `${stats.today_homework_count} 项` },
                  { key: '2', label: '已完成', value: `${stats.today_homework_completed} 人次` },
                  { key: '3', label: '未完成', value: `${stats.today_homework_total_records - stats.today_homework_completed} 人次` },
                  { key: '4', label: '完成率', value: `${Number((stats.today_homework_rate * 100).toFixed(1))}%` },
                ]}
                columns={[
                  { title: '指标', dataIndex: 'label', key: 'label' },
                  { title: '数值', dataIndex: 'value', key: 'value' },
                ]}
                pagination={false}
                size="small"
              />
            ) : (
              <Empty description="今日无作业" />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title="今日考勤情况" size="small">
            {stats.today_attendance_normal > 0 || stats.today_attendance_late > 0 ? (
              <Table
                dataSource={[
                  { key: '1', label: '正常', value: stats.today_attendance_normal },
                  { key: '2', label: '迟到', value: stats.today_attendance_late },
                  { key: '3', label: '早退', value: stats.today_attendance_early },
                  { key: '4', label: '请假', value: stats.today_attendance_leave },
                  { key: '5', label: '旷课', value: stats.today_attendance_absent },
                ]}
                columns={[
                  { title: '状态', dataIndex: 'label', key: 'label' },
                  { title: '人数', dataIndex: 'value', key: 'value' },
                ]}
                pagination={false}
                size="small"
              />
            ) : (
              <Empty description="今日暂无考勤数据" />
            )}
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={8}>
          <Card title="作业趋势" size="small">
            {homeworkTrend.length > 0 ? (
              <Suspense fallback={<ChartFallback />}>
                <LazyLine
                  data={homeworkTrend.map((item) => ({
                    date: item.publish_date,
                    value: Number((item.completion_rate * 100).toFixed(1)),
                  }))}
                  xField="date"
                  yField="value"
                  point={{ size: 3 }}
                />
              </Suspense>
            ) : (
              <Empty description="暂无作业趋势" />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title="考勤趋势" size="small">
            {attendanceTrend.length > 0 ? (
              <Suspense fallback={<ChartFallback />}>
                <LazyLine
                  data={attendanceTrend.map((item) => ({
                    date: item.attendance_date,
                    value: Number((item.normal_rate * 100).toFixed(1)),
                  }))}
                  xField="date"
                  yField="value"
                  point={{ size: 3 }}
                />
              </Suspense>
            ) : (
              <Empty description="暂无考勤趋势" />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title="成绩趋势" size="small">
            {scoreTrend.length > 0 ? (
              <Suspense fallback={<ChartFallback />}>
                <LazyLine
                  data={scoreTrend.map((item) => ({
                    exam: item.exam_name,
                    value: Number(item.avg_score.toFixed(1)),
                  }))}
                  xField="exam"
                  yField="value"
                  point={{ size: 3 }}
                />
              </Suspense>
            ) : (
              <Empty description="暂无成绩趋势" />
            )}
          </Card>
        </Col>
      </Row>

      {stats.focus_students.length > 0 && (
        <Card title="重点关注学生" size="small" style={{ marginTop: 16 }}>
          <List
            dataSource={stats.focus_students}
            renderItem={(item) => (
              <List.Item
                actions={[
                  <Button type="link" onClick={() => navigate(`/students/${item.id}`)}>
                    查看详情
                  </Button>,
                  <Button type="link" onClick={() => goToStudents(true)}>
                    查看关注名单
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  avatar={<WarningOutlined style={{ color: '#faad14', fontSize: 20 }} />}
                  title={item.name}
                  description={item.reason}
                />
              </List.Item>
            )}
          />
        </Card>
      )}
    </div>
  );
}

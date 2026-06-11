import { useState } from 'react';
import { Card, Table, Tabs, DatePicker, Select, Typography, Spin, Empty, Space } from 'antd';
import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { statisticsService, cohortService } from '@/services';
import dayjs from 'dayjs';
import type { Cohort } from '@/types';

const { Title } = Typography;
const { RangePicker } = DatePicker;

function HomeworkStatsTab({ cohortId }: { cohortId: number }) {
  const { data, isLoading } = useQuery({
    queryKey: ['stats', 'homework', cohortId],
    queryFn: () => statisticsService.homeworkStats(cohortId),
    enabled: !!cohortId,
  });

  if (isLoading) return <Spin />;
  if (!data) return <Empty description="暂无数据" />;

  return (
    <div>
      <Card size="small" style={{ marginBottom: 16 }}>
        <Space>
          <span>总作业数：<strong>{data.total}</strong></span>
          <span>平均完成率：<strong>{(data.avg_rate * 100).toFixed(1)}%</strong></span>
          <span>总未交次数：<strong>{data.total_incomplete}</strong></span>
        </Space>
      </Card>
      {data.consecutive_incomplete.length > 0 && (
        <Table
          title={() => '连续未交学生'}
          dataSource={data.consecutive_incomplete}
          columns={[
            { title: '姓名', dataIndex: 'student_name' },
            { title: '学号', dataIndex: 'student_no' },
            { title: '连续未交次数', dataIndex: 'count', render: (v: number) => <span style={{ color: '#ff4d4f' }}>{v} 次</span> },
          ]}
          rowKey="student_id"
          size="small"
          pagination={false}
        />
      )}
    </div>
  );
}

function AttendanceStatsTab({ cohortId }: { cohortId: number }) {
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>([
    dayjs().startOf('month'),
    dayjs().endOf('month'),
  ]);

  const { data, isLoading } = useQuery({
    queryKey: ['stats', 'attendance', cohortId, dateRange[0].format('YYYY-MM-DD'), dateRange[1].format('YYYY-MM-DD')],
    queryFn: () => statisticsService.attendanceStats(cohortId, dateRange[0].format('YYYY-MM-DD'), dateRange[1].format('YYYY-MM-DD')),
    enabled: !!cohortId,
  });

  return (
    <div>
      <RangePicker
        value={dateRange}
        onChange={(dates) => dates && setDateRange([dates[0]!, dates[1]!])}
        style={{ marginBottom: 16 }}
      />
      {isLoading ? <Spin /> : data ? (
        <Table
          dataSource={data.records}
          columns={[
            { title: '姓名', dataIndex: 'student_name' },
            { title: '学号', dataIndex: 'student_no' },
            { title: '总天数', dataIndex: 'total' },
            { title: '正常', dataIndex: 'normal' },
            { title: '迟到', dataIndex: 'late' },
            { title: '早退', dataIndex: 'early' },
            { title: '请假', dataIndex: 'leave' },
            { title: '旷课', dataIndex: 'absent' },
            { title: '出勤率', dataIndex: 'rate', render: (v: number) => `${(v * 100).toFixed(1)}%` },
          ]}
          rowKey="student_id"
          size="small"
          pagination={{ pageSize: 20 }}
        />
      ) : <Empty />}
    </div>
  );
}

function ScoreStatsTab({ cohortId }: { cohortId: number }) {
  const { data, isLoading } = useQuery({
    queryKey: ['stats', 'scores', cohortId],
    queryFn: () => statisticsService.scoreStats(cohortId),
    enabled: !!cohortId,
  });

  if (isLoading) return <Spin />;
  if (!data) return <Empty />;

  return (
    <div>
      <Card size="small" style={{ marginBottom: 16 }}>
        考试数量：<strong>{data.exams_count}</strong> | 科目数量：<strong>{data.subjects_count}</strong>
      </Card>
      <Table
        dataSource={data.records}
        columns={[
          { title: '考试', dataIndex: 'exam_name' },
          { title: '科目', dataIndex: 'subject_name' },
          { title: '平均分', dataIndex: 'avg_score', render: (v: number) => v.toFixed(1) },
          { title: '最高分', dataIndex: 'max_score' },
          { title: '最低分', dataIndex: 'min_score' },
        ]}
        rowKey={(_, idx) => String(idx)}
        size="small"
        pagination={false}
      />
    </div>
  );
}

export default function StatisticsPage() {
  const { currentCohort } = useAppStore();
  const [selectedCohortId, setSelectedCohortId] = useState<number | undefined>();
  const activeCohortId = selectedCohortId ?? currentCohort?.id;

  const { data: cohorts } = useQuery({
    queryKey: ['cohorts'],
    queryFn: () => cohortService.list(),
  });

  const tabItems = activeCohortId ? [
    { key: 'homework', label: '作业统计', children: <HomeworkStatsTab cohortId={activeCohortId} /> },
    { key: 'attendance', label: '考勤统计', children: <AttendanceStatsTab cohortId={activeCohortId} /> },
    { key: 'scores', label: '成绩统计', children: <ScoreStatsTab cohortId={activeCohortId} /> },
  ] : [
    { key: 'empty', label: '暂无数据', children: <Empty description="请先创建或选择一个届次" /> },
  ];

  return (
    <div>
      <div className="page-header">
        <Title level={4}>数据统计</Title>
        <Space>
          <span>选择届次：</span>
          <Select
            style={{ width: 260 }}
            placeholder="选择届次（默认当前）"
            allowClear
            value={selectedCohortId}
            onChange={(val) => setSelectedCohortId(val)}
            onClear={() => setSelectedCohortId(undefined)}
            options={(cohorts || []).map((c: Cohort) => ({
              value: c.id,
              label: `${c.cohort_name} ${c.class_name} ${c.status === '已归档' ? '(已归档)' : ''}`,
            }))}
          />
          <span style={{ color: '#888', fontSize: 12 }}>
            {selectedCohortId ? '已选择指定届次' : `当前届次: ${currentCohort?.cohort_name ?? '无'} ${currentCohort?.class_name ?? ''}`}
          </span>
        </Space>
      </div>
      <Card>
        <Tabs items={tabItems} />
      </Card>
    </div>
  );
}

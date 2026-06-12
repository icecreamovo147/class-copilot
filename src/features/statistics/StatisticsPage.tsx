import { useMemo, useState } from 'react';
import { Line } from '@ant-design/charts';
import { Alert, Button, Card, DatePicker, Empty, Select, Space, Spin, Statistic, Table, Tabs, Tag, Typography, message } from 'antd';
import { DownloadOutlined } from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { cohortService, statisticsService } from '@/services';
import dayjs from 'dayjs';
import type { Cohort } from '@/types';

const { Title, Paragraph } = Typography;
const { RangePicker } = DatePicker;

function HomeworkStatsTab({ cohortId }: { cohortId: number }) {
  const { data, isLoading } = useQuery({
    queryKey: ['stats', 'homework', cohortId],
    queryFn: () => statisticsService.homeworkStats(cohortId),
    enabled: !!cohortId,
  });

  const { data: trendData, isLoading: trendLoading } = useQuery({
    queryKey: ['stats', 'homeworkTrend', cohortId],
    queryFn: () => statisticsService.homeworkTrend(cohortId),
    enabled: !!cohortId,
  });

  if (isLoading || trendLoading) return <Spin />;
  if (!data) return <Empty description="暂无数据" />;

  return (
    <Space direction="vertical" style={{ width: '100%' }} size={16}>
      <Space wrap>
        <Statistic title="总作业数" value={data.total} />
        <Statistic title="平均完成率" value={Number((data.avg_rate * 100).toFixed(1))} suffix="%" />
        <Statistic title="总未交次数" value={data.total_incomplete} />
      </Space>
      {trendData && trendData.length > 0 ? (
        <Card size="small" title="作业完成率趋势">
          <Line
            data={trendData.map((item) => ({
              date: item.publish_date,
              value: Number((item.completion_rate * 100).toFixed(1)),
            }))}
            xField="date"
            yField="value"
            point={{ size: 4 }}
            axis={{ y: { title: '完成率(%)' } }}
          />
        </Card>
      ) : (
        <Empty description="暂无作业趋势数据" />
      )}
      <Table
        title={() => '连续未交学生'}
        dataSource={data.consecutive_incomplete}
        columns={[
          { title: '姓名', dataIndex: 'student_name' },
          { title: '学号', dataIndex: 'student_no' },
          { title: '连续未交次数', dataIndex: 'count', render: (v: number) => <Tag color="red">{v} 次</Tag> },
        ]}
        rowKey="student_id"
        size="small"
        pagination={false}
      />
    </Space>
  );
}

function AttendanceStatsTab({ cohortId }: { cohortId: number }) {
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>([
    dayjs().startOf('month'),
    dayjs().endOf('month'),
  ]);

  const startDate = dateRange[0].format('YYYY-MM-DD');
  const endDate = dateRange[1].format('YYYY-MM-DD');

  const { data, isLoading } = useQuery({
    queryKey: ['stats', 'attendance', cohortId, startDate, endDate],
    queryFn: () => statisticsService.attendanceStats(cohortId, startDate, endDate),
    enabled: !!cohortId,
  });

  const { data: trendData, isLoading: trendLoading } = useQuery({
    queryKey: ['stats', 'attendanceTrend', cohortId, startDate, endDate],
    queryFn: () => statisticsService.attendanceTrend(cohortId, startDate, endDate),
    enabled: !!cohortId,
  });

  const chartData = useMemo(
    () =>
      (trendData || []).flatMap((item) => [
        { date: item.attendance_date, type: '正常率', value: Number((item.normal_rate * 100).toFixed(1)) },
        { date: item.attendance_date, type: '迟到', value: item.late_count },
        { date: item.attendance_date, type: '旷课', value: item.absent_count },
      ]),
    [trendData],
  );

  return (
    <Space direction="vertical" style={{ width: '100%' }} size={16}>
      <RangePicker
        value={dateRange}
        onChange={(dates) => dates && setDateRange([dates[0]!, dates[1]!])}
      />
      {trendLoading ? <Spin /> : chartData.length > 0 ? (
        <Card size="small" title="考勤趋势">
          <Line data={chartData} xField="date" yField="value" colorField="type" point={{ size: 3 }} />
        </Card>
      ) : (
        <Empty description="暂无考勤趋势数据" />
      )}
      {isLoading ? (
        <Spin />
      ) : data ? (
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
      ) : (
        <Empty />
      )}
    </Space>
  );
}

function ScoreStatsTab({ cohortId }: { cohortId: number }) {
  const { data, isLoading } = useQuery({
    queryKey: ['stats', 'scores', cohortId],
    queryFn: () => statisticsService.scoreStats(cohortId),
    enabled: !!cohortId,
  });

  const { data: trendData, isLoading: trendLoading } = useQuery({
    queryKey: ['stats', 'scoreTrend', cohortId],
    queryFn: () => statisticsService.scoreTrend(cohortId),
    enabled: !!cohortId,
  });

  if (isLoading || trendLoading) return <Spin />;
  if (!data) return <Empty />;

  return (
    <Space direction="vertical" style={{ width: '100%' }} size={16}>
      <Space wrap>
        <Statistic title="考试数量" value={data.exams_count} />
        <Statistic title="科目数量" value={data.subjects_count} />
      </Space>
      {trendData && trendData.length > 0 ? (
        <Card size="small" title="成绩趋势">
          <Line
            data={trendData.map((item) => ({
              exam: item.exam_name,
              score: Number(item.avg_score.toFixed(1)),
              subject: item.subject_name,
            }))}
            xField="exam"
            yField="score"
            colorField="subject"
            point={{ size: 3 }}
          />
        </Card>
      ) : (
        <Empty description="暂无成绩趋势数据" />
      )}
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
    </Space>
  );
}

function CrossCohortTab() {
  const [selectedIds, setSelectedIds] = useState<number[]>([]);

  const { data: cohorts = [] } = useQuery({
    queryKey: ['cohorts'],
    queryFn: () => cohortService.list(),
  });

  const { data, isLoading } = useQuery({
    queryKey: ['stats', 'crossCohort', selectedIds],
    queryFn: () => statisticsService.crossCohortComparison(selectedIds),
    enabled: selectedIds.length >= 2,
  });

  const handleExport = async (format: 'xlsx' | 'pdf') => {
    if (selectedIds.length < 2) {
      message.warning('至少选择两个届次');
      return;
    }
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const ext = format === 'pdf' ? 'pdf' : 'xlsx';
      const filePath = await save({
        filters: [{ name: format === 'pdf' ? 'PDF' : 'Excel', extensions: [ext] }],
        defaultPath: `跨届对比统计.${ext}`,
      });
      if (filePath) {
        if (format === 'pdf') {
          await statisticsService.exportCrossCohortComparisonPdf(selectedIds, filePath);
        } else {
          await statisticsService.exportCrossCohortComparison(selectedIds, filePath);
        }
        message.success('对比结果已导出');
      }
    } catch {
      message.error('导出失败');
    }
  };

  return (
    <Space direction="vertical" style={{ width: '100%' }} size={16}>
      <Space wrap>
        <Select
          mode="multiple"
          style={{ width: 420 }}
          placeholder="选择两个或多个届次"
          value={selectedIds}
          onChange={setSelectedIds}
          options={cohorts.map((c: Cohort) => ({
            value: c.id,
            label: `${c.cohort_name} ${c.class_name} ${c.status === '已归档' ? '(已归档)' : ''}`,
          }))}
        />
        <Button icon={<DownloadOutlined />} onClick={() => handleExport('xlsx')} disabled={selectedIds.length < 2}>
          导出对比 Excel
        </Button>
        <Button icon={<DownloadOutlined />} onClick={() => handleExport('pdf')} disabled={selectedIds.length < 2}>
          导出对比 PDF
        </Button>
      </Space>
      <Paragraph type="secondary" style={{ margin: 0 }}>
        缺失成绩数据的届次会单独标识，不参与平均成绩误判。
      </Paragraph>
      {selectedIds.length < 2 ? (
        <Alert type="info" showIcon message="请选择两个或多个届次进行横向对比" />
      ) : isLoading ? (
        <Spin />
      ) : data && data.length > 0 ? (
        <Table
          dataSource={data}
          rowKey="cohort_id"
          size="small"
          pagination={false}
          columns={[
            { title: '届次', render: (_, row) => `${row.cohort_name} ${row.class_name}` },
            { title: '状态', dataIndex: 'status', render: (value: string) => <Tag>{value}</Tag> },
            { title: '人数', dataIndex: 'student_count' },
            { title: '作业完成率', dataIndex: 'homework_completion_rate', render: (v: number) => `${(v * 100).toFixed(1)}%` },
            { title: '出勤率', dataIndex: 'attendance_rate', render: (v: number) => `${(v * 100).toFixed(1)}%` },
            {
              title: '平均成绩',
              render: (_, row) => (row.missing_score_data ? <Tag color="gold">缺失</Tag> : row.avg_score.toFixed(1)),
            },
            { title: '奖惩次数', dataIndex: 'behavior_count' },
            { title: '奖惩分值', dataIndex: 'behavior_score_total' },
          ]}
        />
      ) : (
        <Empty description="暂无可对比数据" />
      )}
    </Space>
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

  const handleExportStatistics = async (format: 'xlsx' | 'pdf') => {
    if (!activeCohortId) {
      message.warning('请先选择届次');
      return;
    }
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const ext = format === 'pdf' ? 'pdf' : 'xlsx';
      const filePath = await save({
        filters: [{ name: format === 'pdf' ? 'PDF' : 'Excel', extensions: [ext] }],
        defaultPath: `统计报表_${activeCohortId}.${ext}`,
      });
      if (filePath) {
        if (format === 'pdf') {
          await statisticsService.exportCohortStatisticsPdf(activeCohortId, filePath);
        } else {
          await statisticsService.exportCohortStatisticsExcel(activeCohortId, filePath);
        }
        message.success('统计报表导出成功');
      }
    } catch {
      message.error('统计报表导出失败');
    }
  };

  const tabItems = activeCohortId
    ? [
        { key: 'homework', label: '作业统计', children: <HomeworkStatsTab cohortId={activeCohortId} /> },
        { key: 'attendance', label: '考勤统计', children: <AttendanceStatsTab cohortId={activeCohortId} /> },
        { key: 'scores', label: '成绩统计', children: <ScoreStatsTab cohortId={activeCohortId} /> },
        { key: 'compare', label: '跨届对比', children: <CrossCohortTab /> },
      ]
    : [{ key: 'empty', label: '暂无数据', children: <Empty description="请先创建或选择一个届次" /> }];

  return (
    <div>
      <div className="page-header">
        <Title level={4}>数据统计</Title>
        <Space wrap>
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
          <Button icon={<DownloadOutlined />} disabled={!activeCohortId} onClick={() => handleExportStatistics('xlsx')}>
            导出统计 Excel
          </Button>
          <Button icon={<DownloadOutlined />} disabled={!activeCohortId} onClick={() => handleExportStatistics('pdf')}>
            导出统计 PDF
          </Button>
        </Space>
      </div>
      <Card>
        <Tabs items={tabItems} />
      </Card>
    </div>
  );
}

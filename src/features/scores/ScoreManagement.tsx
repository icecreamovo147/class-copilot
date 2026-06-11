import { useState } from 'react';
import { Card, Table, Button, Modal, Form, Input, Select, DatePicker, message, Space, Tag, Typography, Tabs, Row, Col, Descriptions, Empty } from 'antd';
import { PlusOutlined, UploadOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { examService, scoreService, studentService, subjectService } from '@/services';
import type { Exam } from '@/types';

const { Title } = Typography;

export default function ScoreManagement() {
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [examModalVisible, setExamModalVisible] = useState(false);
  const [editingExam, setEditingExam] = useState<Exam | null>(null);
  const [selectedExamId, setSelectedExamId] = useState<number | null>(null);
  const [selectedSubjectId, setSelectedSubjectId] = useState<number | null>(null);
  const [scoreModalVisible, setScoreModalVisible] = useState(false);
  const [examForm] = Form.useForm();
  const [scoreForm] = Form.useForm();

  const { data: exams, isLoading: examsLoading } = useQuery({
    queryKey: ['exams', currentCohort?.id],
    queryFn: () => examService.list(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data: subjects } = useQuery({
    queryKey: ['subjects'],
    queryFn: () => subjectService.list(),
  });

  const { data: students } = useQuery({
    queryKey: ['allStudents', currentCohort?.id],
    queryFn: () => studentService.getAll(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data: scores, isLoading: scoresLoading } = useQuery({
    queryKey: ['scores', selectedExamId, selectedSubjectId],
    queryFn: () => scoreService.getByExam(selectedExamId!, selectedSubjectId!),
    enabled: !!selectedExamId && !!selectedSubjectId,
  });

  const { data: scoreStats } = useQuery({
    queryKey: ['scoreStats', selectedExamId, selectedSubjectId],
    queryFn: () => scoreService.statistics(selectedExamId!, selectedSubjectId!),
    enabled: !!selectedExamId && !!selectedSubjectId,
  });

  const { data: rankings } = useQuery({
    queryKey: ['rankings', selectedExamId],
    queryFn: () => scoreService.rankings(selectedExamId!),
    enabled: !!selectedExamId,
  });

  const createExamMutation = useMutation({
    mutationFn: (data: Partial<Exam>) => examService.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['exams'] });
      message.success('考试创建成功');
      setExamModalVisible(false);
      examForm.resetFields();
    },
    onError: (err: Error) => message.error(err.message),
  });

  const saveScoresMutation = useMutation({
    mutationFn: (scoresData: Array<{ student_id: number; score_value: number | null }>) =>
      scoreService.save(selectedExamId!, selectedSubjectId!, scoresData),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scores'] });
      queryClient.invalidateQueries({ queryKey: ['scoreStats'] });
      queryClient.invalidateQueries({ queryKey: ['rankings'] });
      message.success('成绩保存成功');
      setScoreModalVisible(false);
    },
    onError: (err: Error) => message.error(err.message),
  });

  const handleCreateExam = async () => {
    const values = await examForm.validateFields();
    createExamMutation.mutate({
      ...values,
      cohort_id: currentCohort!.id,
      exam_date: values.exam_date?.format('YYYY-MM-DD'),
    });
  };

  const handleSaveScores = async () => {
    const values = await scoreForm.validateFields();
    const scoresData = Object.entries(values)
      .filter(([key]) => key.startsWith('score_'))
      .map(([key, val]) => ({
        student_id: parseInt(key.replace('score_', '')),
        score_value: val !== undefined && val !== null && val !== '' ? Number(val) : null,
      }));
    saveScoresMutation.mutate(scoresData);
  };

  const handleImportScores = async () => {
    if (!selectedExamId || !selectedSubjectId) return;
    try {
      const selected = await import('@tauri-apps/plugin-dialog').then((m) =>
        m.open({
          multiple: false,
          filters: [{ name: 'Excel', extensions: ['xlsx', 'xls'] }],
        })
      );
      if (!selected || typeof selected !== 'string') return;
      const result = await scoreService.importExcel(selectedExamId, selectedSubjectId, selected);
      if (result.errors.length > 0) {
        message.warning(`成功导入 ${result.success} 条，${result.errors.length} 条失败`);
      } else {
        message.success(`成功导入 ${result.success} 条记录`);
      }
      queryClient.invalidateQueries({ queryKey: ['scores'] });
      queryClient.invalidateQueries({ queryKey: ['scoreStats'] });
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '导入失败');
    }
  };

  const examColumns = [
    { title: '考试名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'exam_type', key: 'exam_type', render: (v: string | null) => v || '-' },
    { title: '考试日期', dataIndex: 'exam_date', key: 'exam_date', render: (v: string | null) => v || '-' },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Exam) => (
        <Button type="link" onClick={() => setSelectedExamId(record.id)}>
          选择
        </Button>
      ),
    },
  ];

  const scoreColumns = [
    { title: '姓名', dataIndex: 'student_name', key: 'student_name' },
    { title: '学号', dataIndex: 'student_no', key: 'student_no' },
    {
      title: '成绩',
      dataIndex: 'score_value',
      key: 'score_value',
      render: (v: number | null) => v !== null ? v : '-',
    },
    {
      title: '排名',
      dataIndex: 'rank_no',
      key: 'rank_no',
      render: (v: number | null) => v ? <Tag color="blue">{v}</Tag> : '-',
    },
  ];

  const scoreDataSource = (students || []).map((s) => {
    const score = (scores || []).find((sc) => sc.student_id === s.id);
    return {
      key: s.id,
      student_name: s.name,
      student_no: s.student_no,
      score_value: score?.score_value ?? null,
      rank_no: score?.rank_no ?? null,
    };
  });

  const tabItems = [
    {
      key: 'exams',
      label: '考试列表',
      children: (
        <div>
          {!isReadonly && (
            <Button type="primary" icon={<PlusOutlined />} onClick={() => { setEditingExam(null); examForm.resetFields(); setExamModalVisible(true); }} style={{ marginBottom: 16 }}>
              创建考试
            </Button>
          )}
          <Table
            dataSource={exams || []}
            columns={examColumns}
            rowKey="id"
            loading={examsLoading}
            pagination={false}
          />
        </div>
      ),
    },
    {
      key: 'scores',
      label: '成绩录入',
      children: selectedExamId ? (
        <div>
          <Space style={{ marginBottom: 16 }}>
            <Select
              style={{ width: 200 }}
              placeholder="选择考试"
              value={selectedExamId}
              onChange={(val) => { setSelectedExamId(val); setSelectedSubjectId(null); }}
              options={(exams || []).map((e) => ({ value: e.id, label: e.name }))}
            />
            <Select
              style={{ width: 200 }}
              placeholder="选择科目"
              value={selectedSubjectId}
              onChange={setSelectedSubjectId}
              options={(subjects || []).map((s) => ({ value: s.id, label: s.name }))}
            />
            {!isReadonly && selectedSubjectId && (
              <>
                <Button
                  type="primary"
                  onClick={() => {
                    scoreForm.resetFields();
                    const initialValues: Record<string, number | null> = {};
                    (students || []).forEach((s) => {
                      const score = (scores || []).find((sc) => sc.student_id === s.id);
                      initialValues[`score_${s.id}`] = score?.score_value ?? null;
                    });
                    scoreForm.setFieldsValue(initialValues);
                    setScoreModalVisible(true);
                  }}
                >
                  录入成绩
                </Button>
                <Button icon={<UploadOutlined />} onClick={handleImportScores}>
                  导入成绩
                </Button>
              </>
            )}
          </Space>

          {selectedSubjectId && (
            <>
              {scoreStats && (
                <Descriptions bordered size="small" style={{ marginBottom: 16 }} column={4}>
                  <Descriptions.Item label="平均分">{scoreStats.avg_score.toFixed(1)}</Descriptions.Item>
                  <Descriptions.Item label="最高分">{scoreStats.max_score}</Descriptions.Item>
                  <Descriptions.Item label="最低分">{scoreStats.min_score}</Descriptions.Item>
                  <Descriptions.Item label="及格率">{(scoreStats.pass_rate * 100).toFixed(1)}%</Descriptions.Item>
                </Descriptions>
              )}
              <Table
                dataSource={scoreDataSource}
                columns={scoreColumns}
                rowKey="key"
                loading={scoresLoading}
                pagination={false}
                size="small"
              />
            </>
          )}
        </div>
      ) : (
        <Empty description="请先在考试列表中选择一个考试" />
      ),
    },
    {
      key: 'rankings',
      label: '排名统计',
      children: (
        <div>
          <Select
            style={{ width: 200, marginBottom: 16 }}
            placeholder="选择考试查看排名"
            value={selectedExamId}
            onChange={setSelectedExamId}
            options={(exams || []).map((e) => ({ value: e.id, label: e.name }))}
          />
          {rankings && (
            <Table
              dataSource={rankings.map((r, idx) => ({ ...r, key: r.student_id, rank: idx + 1 }))}
              columns={[
                { title: '排名', dataIndex: 'rank', key: 'rank', render: (v: number) => <Tag color="blue">{v}</Tag> },
                { title: '姓名', dataIndex: 'student_name', key: 'student_name' },
                { title: '学号', dataIndex: 'student_no', key: 'student_no' },
                { title: '总分', dataIndex: 'total_score', key: 'total_score', render: (v: number) => v.toFixed(1) },
              ]}
              rowKey="key"
              size="small"
              pagination={false}
            />
          )}
          {!selectedExamId && <Empty description="请选择考试" />}
        </div>
      ),
    },
  ];

  return (
    <div>
      <div className="page-header">
        <Title level={4}>成绩管理</Title>
      </div>

      <Card>
        <Tabs items={tabItems} />
      </Card>

      <Modal
        title={editingExam ? '编辑考试' : '创建考试'}
        open={examModalVisible}
        onOk={handleCreateExam}
        onCancel={() => setExamModalVisible(false)}
        confirmLoading={createExamMutation.isPending}
      >
        <Form form={examForm} layout="vertical">
          <Form.Item name="name" label="考试名称" rules={[{ required: true, message: '请输入考试名称' }]}>
            <Input placeholder="如：期中考试" />
          </Form.Item>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="exam_type" label="考试类型">
                <Select allowClear options={[
                  { value: '月考', label: '月考' },
                  { value: '期中', label: '期中' },
                  { value: '期末', label: '期末' },
                  { value: '模拟', label: '模拟' },
                ]} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="exam_date" label="考试日期">
                <DatePicker style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="录入成绩"
        open={scoreModalVisible}
        onOk={handleSaveScores}
        onCancel={() => setScoreModalVisible(false)}
        confirmLoading={saveScoresMutation.isPending}
        width={600}
      >
        <Form form={scoreForm} layout="vertical">
          <Table
            dataSource={(students || []).map((s) => ({
              key: s.id,
              student_name: s.name,
              student_no: s.student_no,
              id: s.id,
            }))}
            columns={[
              { title: '姓名', dataIndex: 'student_name', key: 'student_name', width: 100 },
              { title: '学号', dataIndex: 'student_no', key: 'student_no', width: 100 },
              {
                title: '成绩',
                key: 'score',
                width: 150,
                render: (_: unknown, record: { id: number }) => (
                  <Form.Item name={`score_${record.id}`} noStyle>
                    <Input type="number" style={{ width: 120 }} placeholder="输入成绩" />
                  </Form.Item>
                ),
              },
            ]}
            pagination={false}
            size="small"
          />
        </Form>
      </Modal>
    </div>
  );
}

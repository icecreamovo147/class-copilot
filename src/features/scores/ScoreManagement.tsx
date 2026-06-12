import { useEffect, useState } from 'react';
import { Alert, Card, Table, Button, Modal, Form, Input, Select, DatePicker, message, Space, Tag, Typography, Tabs, Row, Col, Descriptions, Empty, Popconfirm } from 'antd';
import { PlusOutlined, UploadOutlined, EditOutlined, DeleteOutlined, DownloadOutlined, SettingOutlined } from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { configService, examService, scoreService, studentService, subjectService } from '@/services';
import { useKeyboardShortcut } from '@/hooks/useKeyboardShortcut';
import type { Exam, ExamSubjectConfig, Subject } from '@/types';
import dayjs from 'dayjs';

const { Title } = Typography;

export default function ScoreManagement() {
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const [examModalVisible, setExamModalVisible] = useState(false);
  const [editingExam, setEditingExam] = useState<Exam | null>(null);
  const [selectedExamId, setSelectedExamId] = useState<number | null>(null);
  const [selectedSubjectId, setSelectedSubjectId] = useState<number | null>(null);
  const [scoreModalVisible, setScoreModalVisible] = useState(false);
  const [subjectModalVisible, setSubjectModalVisible] = useState(false);
  const [configModalVisible, setConfigModalVisible] = useState(false);
  const [scorePreviewVisible, setScorePreviewVisible] = useState(false);
  const [editingSubject, setEditingSubject] = useState<Subject | null>(null);
  const [scoreImportFilePath, setScoreImportFilePath] = useState<string | null>(null);
  const [scorePreviewData, setScorePreviewData] = useState<{
    total_rows: number;
    valid_rows: number;
    error_rows: number;
    rows: Array<Record<string, unknown>>;
    errors: string[];
    warnings: string[];
  } | null>(null);
  const [examForm] = Form.useForm();
  const [scoreForm] = Form.useForm();
  const [subjectForm] = Form.useForm();
  const [configForm] = Form.useForm();

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

  const { data: examSubjectConfigs = [] } = useQuery({
    queryKey: ['examSubjectConfigs', selectedExamId],
    queryFn: () => examService.getSubjectConfigs(selectedExamId!),
    enabled: !!selectedExamId,
  });

  useEffect(() => {
    if (!configModalVisible) return;
    const rows = (subjects || []).map((subject) => {
      const existing = examSubjectConfigs.find((config) => config.subject_id === subject.id);
      return {
        enabled: !!existing,
        subject_id: subject.id,
        subject_name: subject.name,
        full_score: existing?.full_score ?? 100,
        pass_score: existing?.pass_score ?? 60,
        excellent_score: existing?.excellent_score ?? 90,
        sort_order: existing?.sort_order ?? subject.sort_order ?? 0,
      };
    });
    configForm.setFieldsValue({ configs: rows });
  }, [configForm, configModalVisible, examSubjectConfigs, subjects]);

  useEffect(() => {
    if (!selectedSubjectId) return;
    const exists = examSubjectConfigs.some((config) => config.subject_id === selectedSubjectId);
    if (!exists) {
      setSelectedSubjectId(null);
    }
  }, [examSubjectConfigs, selectedSubjectId]);

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

  const updateExamMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<Exam> }) => examService.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['exams'] });
      message.success('考试更新成功');
      setExamModalVisible(false);
      examForm.resetFields();
    },
    onError: (err: Error) => message.error(err.message),
  });

  const deleteExamMutation = useMutation({
    mutationFn: (id: number) => examService.delete(id),
    onSuccess: (_: void, id: number) => {
      queryClient.invalidateQueries({ queryKey: ['exams'] });
      message.success('考试已删除');
      if (selectedExamId === id) { setSelectedExamId(null); setSelectedSubjectId(null); }
    },
    onError: (err: Error) => message.error(err.message),
  });

  const createSubjectMutation = useMutation({
    mutationFn: (data: Partial<Subject>) => subjectService.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subjects'] });
      message.success('科目创建成功');
      setSubjectModalVisible(false);
      subjectForm.resetFields();
    },
    onError: (err: Error) => message.error(err.message),
  });

  const updateSubjectMutation = useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<Subject> }) => subjectService.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subjects'] });
      message.success('科目更新成功');
      setSubjectModalVisible(false);
      subjectForm.resetFields();
    },
    onError: (err: Error) => message.error(err.message),
  });

  const deleteSubjectMutation = useMutation({
    mutationFn: (id: number) => subjectService.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subjects'] });
      message.success('科目已删除');
    },
    onError: (err: Error) => message.error(err.message),
  });

  const toggleSubjectActiveMutation = useMutation({
    mutationFn: ({ id, is_active }: { id: number; is_active: boolean }) =>
      subjectService.update(id, { is_active }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subjects'] });
      message.success('科目状态已更新');
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

  const saveConfigMutation = useMutation({
    mutationFn: (configs: ExamSubjectConfig[]) => examService.saveSubjectConfigs(selectedExamId!, configs),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['examSubjectConfigs'] });
      queryClient.invalidateQueries({ queryKey: ['scoreStats'] });
      message.success('考试科目配置已保存');
      setConfigModalVisible(false);
    },
    onError: (err: Error) => message.error(err.message),
  });

  const handleCreateExam = async () => {
    const values = await examForm.validateFields();
    const payload = {
      ...values,
      cohort_id: currentCohort!.id,
      exam_date: values.exam_date?.format('YYYY-MM-DD'),
    };
    if (editingExam) {
      updateExamMutation.mutate({ id: editingExam.id, data: payload });
    } else {
      createExamMutation.mutate(payload);
    }
  };

  const handleEditExam = (exam: Exam) => {
    setEditingExam(exam);
    examForm.setFieldsValue({
      name: exam.name,
      exam_type: exam.exam_type,
      exam_date: exam.exam_date ? dayjs(exam.exam_date) : null,
      remark: exam.remark,
    });
    setExamModalVisible(true);
  };

  const handleDeleteExam = (id: number) => {
    deleteExamMutation.mutate(id);
  };

  const handleOpenConfig = (examId: number) => {
    const existingConfigs = selectedExamId === examId ? examSubjectConfigs : [];
    setSelectedExamId(examId);
    const rows = (subjects || []).map((subject) => {
      const existing = existingConfigs.find((config) => config.subject_id === subject.id);
      return {
        enabled: !!existing,
        subject_id: subject.id,
        subject_name: subject.name,
        full_score: existing?.full_score ?? 100,
        pass_score: existing?.pass_score ?? 60,
        excellent_score: existing?.excellent_score ?? 90,
        sort_order: existing?.sort_order ?? subject.sort_order ?? 0,
      };
    });
    configForm.setFieldsValue({ configs: rows });
    setConfigModalVisible(true);
  };

  const handleCreateSubject = async () => {
    const values = await subjectForm.validateFields();
    if (editingSubject) {
      updateSubjectMutation.mutate({ id: editingSubject.id, data: values });
    } else {
      createSubjectMutation.mutate(values);
    }
  };

  const handleEditSubject = (subject: Subject) => {
    setEditingSubject(subject);
    subjectForm.setFieldsValue({
      name: subject.name,
      sort_order: subject.sort_order,
      is_active: subject.is_active,
      remark: subject.remark,
    });
    setSubjectModalVisible(true);
  };

  const handleDeleteSubject = (id: number) => {
    deleteSubjectMutation.mutate(id);
  };

  const handleToggleSubjectActive = (subject: Subject) => {
    toggleSubjectActiveMutation.mutate({ id: subject.id, is_active: !subject.is_active });
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
      const preview = await scoreService.previewExcel(selectedExamId, selectedSubjectId, selected);
      setScoreImportFilePath(selected);
      setScorePreviewData(preview);
      setScorePreviewVisible(true);
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '导入失败');
    }
  };

  const handleConfirmImportScores = async () => {
    if (!selectedExamId || !selectedSubjectId || !scoreImportFilePath) return;
    try {
      const result = await scoreService.importExcel(selectedExamId, selectedSubjectId, scoreImportFilePath);
      if (result.errors.length > 0) {
        message.warning(`成功导入 ${result.success} 条，${result.errors.length} 条失败`);
      } else if (result.warnings.length > 0) {
        message.warning(`成功导入 ${result.success} 条，存在 ${result.warnings.length} 条覆盖提醒`);
      } else {
        message.success(`成功导入 ${result.success} 条记录`);
      }
      queryClient.invalidateQueries({ queryKey: ['scores'] });
      queryClient.invalidateQueries({ queryKey: ['scoreStats'] });
      queryClient.invalidateQueries({ queryKey: ['rankings'] });
      setScorePreviewVisible(false);
      setScorePreviewData(null);
      setScoreImportFilePath(null);
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '导入失败');
    }
  };

  const handleDownloadScoreTemplate = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: 'score_导入模板.xlsx',
      });
      if (!filePath) return;
      await configService.downloadTemplate('score', filePath);
      message.success('成绩模板下载成功');
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '模板下载失败');
    }
  };

  const handleSaveExamSubjectConfigs = async () => {
    const values = await configForm.validateFields();
    const configs = (values.configs || [])
      .filter((item: { enabled?: boolean }) => item.enabled)
      .map((item: ExamSubjectConfig) => ({
        subject_id: item.subject_id,
        full_score: Number(item.full_score),
        pass_score: Number(item.pass_score),
        excellent_score: Number(item.excellent_score),
        sort_order: Number(item.sort_order ?? 0),
      }));
    saveConfigMutation.mutate(configs);
  };

  const handleExportScores = async () => {
    if (!selectedExamId || !selectedSubjectId) return;
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const exam = (exams || []).find((item) => item.id === selectedExamId);
      const subject = (subjects || []).find((item) => item.id === selectedSubjectId);
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: `${exam?.name || '考试'}_${subject?.name || '科目'}_成绩.xlsx`,
      });
      if (!filePath) return;
      await scoreService.exportExcel(selectedExamId, selectedSubjectId, filePath);
      message.success('成绩导出成功');
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '导出失败');
    }
  };

  useKeyboardShortcut(
    'n',
    () => {
      if (isReadonly) return;
      setEditingExam(null);
      examForm.resetFields();
      setExamModalVisible(true);
    },
    { ctrlOrMeta: true, enabled: !isReadonly },
  );

  useKeyboardShortcut('s', handleSaveScores, {
    ctrlOrMeta: true,
    enabled: scoreModalVisible && !saveScoresMutation.isPending,
  });

  const examColumns = [
    { title: '考试名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'exam_type', key: 'exam_type', render: (v: string | null) => v || '-' },
    { title: '考试日期', dataIndex: 'exam_date', key: 'exam_date', render: (v: string | null) => v || '-' },
    {
      title: '操作',
      key: 'action',
      width: 320,
      render: (_: unknown, record: Exam) => (
        <Space size="small">
          <Button type="link" size="small" onClick={() => setSelectedExamId(record.id)}>选择</Button>
          {!isReadonly && (
            <Button type="link" size="small" icon={<SettingOutlined />} onClick={() => handleOpenConfig(record.id)}>
              配置科目
            </Button>
          )}
          {!isReadonly && (
            <>
              <Button type="link" size="small" icon={<EditOutlined />} onClick={() => handleEditExam(record)}>编辑</Button>
              <Popconfirm title="删除考试会同时删除该考试的所有成绩数据，确认删除？" onConfirm={() => handleDeleteExam(record.id)}>
                <Button type="link" size="small" danger icon={<DeleteOutlined />}>删除</Button>
              </Popconfirm>
            </>
          )}
        </Space>
      ),
    },
  ];

  const subjectColumns = [
    { title: '科目名称', dataIndex: 'name', key: 'name' },
    { title: '排序', dataIndex: 'sort_order', key: 'sort_order', width: 80 },
    {
      title: '状态',
      dataIndex: 'is_active',
      key: 'is_active',
      width: 90,
      render: (value: boolean) => <Tag color={value ? 'green' : 'default'}>{value ? '启用中' : '已停用'}</Tag>,
    },
    { title: '备注', dataIndex: 'remark', key: 'remark', ellipsis: true, render: (v: string | null) => v || '-' },
    {
      title: '操作',
      key: 'action',
      width: 240,
      render: (_: unknown, record: Subject) => (
        <Space size="small">
          <Button type="link" size="small" icon={<EditOutlined />} onClick={() => handleEditSubject(record)}>编辑</Button>
          <Popconfirm
            title={record.is_active ? '停用后历史数据会保留，但新录入时不再默认可选，确认停用？' : '确认重新启用该科目？'}
            onConfirm={() => handleToggleSubjectActive(record)}
          >
            <Button type="link" size="small">{record.is_active ? '停用' : '启用'}</Button>
          </Popconfirm>
          {!record.is_active && (
            <Popconfirm title="仅未被历史作业或成绩引用的停用科目才允许删除，确认删除？" onConfirm={() => handleDeleteSubject(record.id)}>
              <Button type="link" size="small" danger icon={<DeleteOutlined />}>删除</Button>
            </Popconfirm>
          )}
          {record.is_active && (
            <Button type="link" size="small" danger disabled icon={<DeleteOutlined />}>删除</Button>
          )}
        </Space>
      ),
    },
  ];

  const selectableSubjects = (subjects || []).filter((subject) => {
    const config = examSubjectConfigs.find((item) => item.subject_id === subject.id);
    if (!config) return false;
    if (subject.is_active) return true;
    return subject.id === selectedSubjectId;
  });

  const selectedSubject = (subjects || []).find((subject) => subject.id === selectedSubjectId);
  const selectedSubjectInactive = !!selectedSubject && !selectedSubject.is_active;

  const subjectSelectOptions = selectableSubjects.map((subject) => {
    const config = examSubjectConfigs.find((item) => item.subject_id === subject.id);
    return {
      value: subject.id,
      label: `${subject.is_active ? subject.name : `${subject.name}（已停用）`} / 满分 ${config?.full_score ?? 100}`,
    };
  });

  const subjectTabAddButton = !isReadonly && (
    <Button
      type="primary"
      icon={<PlusOutlined />}
      onClick={() => {
        setEditingSubject(null);
        subjectForm.resetFields();
        subjectForm.setFieldsValue({ is_active: true, sort_order: 0 });
        setSubjectModalVisible(true);
      }}
      style={{ marginBottom: 16 }}
    >
      新增科目
    </Button>
  );

  const subjectStatusHint = selectedSubjectInactive ? (
    <Tag color="orange">当前科目已停用，仅保留历史查看与导出</Tag>
  ) : null;

  const scoreActionButtons = !isReadonly && selectedSubjectId && selectedSubject?.is_active ? (
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
      <Button icon={<DownloadOutlined />} onClick={handleDownloadScoreTemplate}>
        下载模板
      </Button>
    </>
  ) : null;

  const scoreExportButton = selectedSubjectId ? (
    <Button icon={<DownloadOutlined />} onClick={handleExportScores}>
      导出成绩
    </Button>
  ) : null;

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
      label: '考试管理',
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
      key: 'subjects',
      label: '科目管理',
      children: (
        <div>
          {subjectTabAddButton}
          <Table
            dataSource={subjects || []}
            columns={subjectColumns}
            rowKey="id"
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
              options={subjectSelectOptions}
            />
            {scoreActionButtons}
            {scoreExportButton}
          </Space>
          {examSubjectConfigs.length === 0 && (
            <Alert
              type="info"
              showIcon
              style={{ marginBottom: 16 }}
              message="当前考试尚未配置科目、满分和统计规则，请先在考试管理中完成配置。"
            />
          )}
          {subjectStatusHint}

          {selectedSubjectId && (
            <>
              {scoreStats && (
                <Descriptions bordered size="small" style={{ marginBottom: 16 }} column={4}>
                  <Descriptions.Item label="满分">{scoreStats.full_score}</Descriptions.Item>
                  <Descriptions.Item label="平均分">{scoreStats.avg_score.toFixed(1)}</Descriptions.Item>
                  <Descriptions.Item label="最高分">{scoreStats.max_score}</Descriptions.Item>
                  <Descriptions.Item label="最低分">{scoreStats.min_score}</Descriptions.Item>
                  <Descriptions.Item label="及格率">{(scoreStats.pass_rate * 100).toFixed(1)}%</Descriptions.Item>
                  <Descriptions.Item label="及格线">{scoreStats.pass_score}</Descriptions.Item>
                  <Descriptions.Item label="优秀率">{(scoreStats.excellent_rate * 100).toFixed(1)}%</Descriptions.Item>
                  <Descriptions.Item label="优秀线">{scoreStats.excellent_score}</Descriptions.Item>
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
              dataSource={rankings.map((r) => ({ ...r, key: r.student_id }))}
              columns={[
                { title: '排名', dataIndex: 'rank_no', key: 'rank_no', render: (v: number) => <Tag color="blue">{v}</Tag> },
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
        onCancel={() => { setExamModalVisible(false); setEditingExam(null); }}
        confirmLoading={createExamMutation.isPending || updateExamMutation.isPending}
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
        title={editingSubject ? '编辑科目' : '新增科目'}
        open={subjectModalVisible}
        onOk={handleCreateSubject}
        onCancel={() => { setSubjectModalVisible(false); setEditingSubject(null); }}
        confirmLoading={createSubjectMutation.isPending || updateSubjectMutation.isPending}
      >
        <Form form={subjectForm} layout="vertical">
          <Form.Item name="name" label="科目名称" rules={[{ required: true, message: '请输入科目名称' }]}>
            <Input placeholder="如：数学" />
          </Form.Item>
          <Form.Item name="sort_order" label="排序序号" tooltip="数字越小越靠前">
            <Input type="number" placeholder="0" />
          </Form.Item>
          <Form.Item name="is_active" label="启用状态" initialValue={true}>
            <Select options={[{ value: true, label: '启用中' }, { value: false, label: '已停用' }]} />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={2} />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="配置考试科目与统计规则"
        open={configModalVisible}
        onOk={handleSaveExamSubjectConfigs}
        onCancel={() => setConfigModalVisible(false)}
        confirmLoading={saveConfigMutation.isPending}
        width={900}
      >
        <Alert
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
          message="为当前考试选择可录入的科目，并配置满分、及格线和优秀线。成绩导入校验与统计会按这里的规则执行。"
        />
        <Form form={configForm} layout="vertical">
          <Form.List name="configs">
            {(fields) => (
              <Table
                pagination={false}
                size="small"
                dataSource={fields}
                rowKey="key"
                columns={[
                  {
                    title: '启用',
                    render: (_, field) => (
                      <Form.Item name={[field.name, 'enabled']} noStyle>
                        <Select
                          options={[
                            { value: true, label: '启用' },
                            { value: false, label: '关闭' },
                          ]}
                          style={{ width: 90 }}
                        />
                      </Form.Item>
                    ),
                  },
                  {
                    title: '科目',
                    render: (_, field) => (
                      <>
                        <Form.Item name={[field.name, 'subject_id']} hidden><Input /></Form.Item>
                        <Form.Item name={[field.name, 'subject_name']} noStyle>
                          <Input disabled />
                        </Form.Item>
                      </>
                    ),
                  },
                  {
                    title: '满分',
                    render: (_, field) => (
                      <Form.Item
                        name={[field.name, 'full_score']}
                        rules={[{ required: true, message: '请输入满分' }]}
                        noStyle
                      >
                        <Input type="number" min={1} />
                      </Form.Item>
                    ),
                  },
                  {
                    title: '及格线',
                    render: (_, field) => (
                      <Form.Item
                        name={[field.name, 'pass_score']}
                        rules={[{ required: true, message: '请输入及格线' }]}
                        noStyle
                      >
                        <Input type="number" min={0} />
                      </Form.Item>
                    ),
                  },
                  {
                    title: '优秀线',
                    render: (_, field) => (
                      <Form.Item
                        name={[field.name, 'excellent_score']}
                        rules={[{ required: true, message: '请输入优秀线' }]}
                        noStyle
                      >
                        <Input type="number" min={0} />
                      </Form.Item>
                    ),
                  },
                  {
                    title: '排序',
                    render: (_, field) => (
                      <Form.Item name={[field.name, 'sort_order']} noStyle>
                        <Input type="number" min={0} />
                      </Form.Item>
                    ),
                  },
                ]}
              />
            )}
          </Form.List>
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

      <Modal
        title="成绩导入预览"
        open={scorePreviewVisible}
        onCancel={() => {
          setScorePreviewVisible(false);
          setScorePreviewData(null);
          setScoreImportFilePath(null);
        }}
        onOk={handleConfirmImportScores}
        okButtonProps={{ disabled: !scorePreviewData || scorePreviewData.valid_rows === 0 || scorePreviewData.error_rows > 0 }}
        width={960}
      >
        {scorePreviewData && (
          <>
            <Space style={{ marginBottom: 12 }}>
              <Tag color="blue">共 {scorePreviewData.total_rows} 条</Tag>
              <Tag color="green">有效 {scorePreviewData.valid_rows} 条</Tag>
              {scorePreviewData.error_rows > 0 && <Tag color="red">错误 {scorePreviewData.error_rows} 条</Tag>}
              {scorePreviewData.warnings.length > 0 && <Tag color="orange">覆盖提醒 {scorePreviewData.warnings.length} 条</Tag>}
            </Space>
            {scorePreviewData.errors.length > 0 && (
              <Alert
                type="error"
                showIcon
                style={{ marginBottom: 12 }}
                message="以下错误会阻止导入，请先修正文件："
                description={
                  <ul style={{ margin: 0, paddingLeft: 16 }}>
                    {scorePreviewData.errors.map((error, index) => <li key={index}>{error}</li>)}
                  </ul>
                }
              />
            )}
            <Table
              dataSource={scorePreviewData.rows}
              rowKey={(row) => String(row.row)}
              size="small"
              pagination={false}
              scroll={{ y: 320 }}
              columns={[
                { title: '行号', dataIndex: 'row', width: 70 },
                { title: '学号', dataIndex: 'student_no', width: 120 },
                { title: '姓名', dataIndex: 'student_name', width: 120, render: (value: string | null) => value || '-' },
                { title: '成绩', dataIndex: 'score_value', width: 100, render: (value: number | null) => value ?? '-' },
                {
                  title: '状态',
                  dataIndex: 'valid',
                  width: 100,
                  render: (value: boolean) => <Tag color={value ? 'green' : 'red'}>{value ? '有效' : '错误'}</Tag>,
                },
                { title: '提醒', dataIndex: 'warning', render: (value: string | null) => value || '-' },
              ]}
            />
          </>
        )}
      </Modal>
    </div>
  );
}

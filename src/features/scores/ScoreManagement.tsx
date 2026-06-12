import { useEffect, useState, useMemo } from 'react';
import {
  Alert, Card, Table, Button, Modal, Form, Input, Select, DatePicker, message,
  Space, Tag, Typography, Row, Col, Popconfirm,
  Dropdown, Divider, Badge, Skeleton,
} from 'antd';
import {
  PlusOutlined, UploadOutlined, EditOutlined, DeleteOutlined,
  DownloadOutlined, SettingOutlined, TrophyOutlined,
  BarChartOutlined, FileTextOutlined, ExperimentOutlined,
  CheckCircleOutlined, ExclamationCircleOutlined,
  BookOutlined, ArrowLeftOutlined,
} from '@ant-design/icons';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { configService, examService, scoreService, studentService, subjectService } from '@/services';
import { useKeyboardShortcut } from '@/hooks/useKeyboardShortcut';
import { useIsDark } from '@/hooks/useTheme';
import type { Exam, ExamSubjectConfig, Subject } from '@/types';
import dayjs from 'dayjs';

const { Title, Text, Paragraph } = Typography;

// ─── sidebar exam list item ────────────────────────────
interface ExamListItemProps {
  exam: Exam;
  isSelected: boolean;
  onSelect: (id: number) => void;
  onEdit: (exam: Exam) => void;
  onDelete: (id: number) => void;
  onConfigure: (examId: number) => void;
  isReadonly: boolean;
  isDark: boolean;
}

function ExamListItem({
  exam, isSelected, onSelect, onEdit, onDelete, onConfigure, isReadonly, isDark: dark,
}: ExamListItemProps) {
  const itemBorder = dark ? '#303030' : '#f0f0f0';
  const itemBg = dark ? '#141414' : '#fff';
  const itemSelectedBg = dark ? '#111d2c' : '#e6f4ff';
  const textHeading = dark ? '#e8e8e8' : '#1a1a2e';
  const textMuted = dark ? '#6b6b6b' : '#999';
  const hoverBorder = dark ? '#434343' : '#d9d9d9';

  return (
    <div
      onClick={() => onSelect(exam.id)}
      style={{
        padding: '12px 16px',
        borderRadius: 8,
        cursor: 'pointer',
        marginBottom: 6,
        border: isSelected ? '2px solid #1677ff' : `1px solid ${itemBorder}`,
        background: isSelected ? itemSelectedBg : itemBg,
        transition: 'all 0.2s',
        position: 'relative',
      }}
      onMouseEnter={(e) => {
        if (!isSelected) (e.currentTarget as HTMLElement).style.borderColor = hoverBorder;
      }}
      onMouseLeave={(e) => {
        if (!isSelected) (e.currentTarget as HTMLElement).style.borderColor = itemBorder;
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontWeight: isSelected ? 600 : 500, fontSize: 14, color: textHeading, marginBottom: 4, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {isSelected && <TrophyOutlined style={{ color: '#1677ff', marginRight: 6 }} />}
            {exam.name}
          </div>
          <Space size={4}>
            {exam.exam_type && <Tag style={{ fontSize: 11, lineHeight: '18px' }}>{exam.exam_type}</Tag>}
            {exam.exam_date ? (
              <Text type="secondary" style={{ fontSize: 12 }}>{exam.exam_date}</Text>
            ) : (
              <Text type="secondary" style={{ fontSize: 12 }}>日期未设定</Text>
            )}
          </Space>
        </div>

        <Dropdown
          menu={{
            items: [
              ...(!isReadonly ? [{ key: 'edit', icon: <EditOutlined />, label: '编辑考试' }] : []),
              ...(!isReadonly ? [{ key: 'configure', icon: <SettingOutlined />, label: '配置科目' }] : []),
              ...(!isReadonly
                ? [{ key: 'delete', icon: <DeleteOutlined />, label: '删除考试', danger: true }]
                : []),
            ],
            onClick: (e) => {
              e.domEvent.stopPropagation();
              if (e.key === 'edit') onEdit(exam);
              if (e.key === 'delete') onDelete(exam.id);
              if (e.key === 'configure') onConfigure(exam.id);
            },
          }}
          trigger={['click']}
        >
          <Button
            type="text"
            size="small"
            icon={<SettingOutlined />}
            onClick={(e) => e.stopPropagation()}
            style={{ color: textMuted, flexShrink: 0 }}
          />
        </Dropdown>
      </div>
    </div>
  );
}

// ─── main component ──────────────────────────────────
export default function ScoreManagement() {
  const queryClient = useQueryClient();
  const { currentCohort, isReadonly } = useAppStore();
  const isDark = useIsDark();

  // ── 暗色适配色值 ──
  const panelBg = isDark ? '#1a1a1a' : '#fafafa';
  const itemBg = isDark ? '#141414' : '#fff';
  const itemBorder = isDark ? '#303030' : '#f0f0f0';
  const itemSelectedBg = isDark ? '#111d2c' : '#e6f4ff';
  const textLabel = isDark ? '#8c8c8c' : '#666';
  const textMuted = isDark ? '#6b6b6b' : '#999';
  const iconMuted = isDark ? '#434343' : '#d9d9d9';

  // ── core selection state ──
  const [selectedExamId, setSelectedExamId] = useState<number | null>(null);
  const [selectedSubjectId, setSelectedSubjectId] = useState<number | null>(null);

  // ── modal states ──
  const [examModalVisible, setExamModalVisible] = useState(false);
  const [editingExam, setEditingExam] = useState<Exam | null>(null);
  const [subjectModalVisible, setSubjectModalVisible] = useState(false);
  const [editingSubject, setEditingSubject] = useState<Subject | null>(null);
  const [configModalVisible, setConfigModalVisible] = useState(false);
  const [scoreModalVisible, setScoreModalVisible] = useState(false);
  const [scorePreviewVisible, setScorePreviewVisible] = useState(false);
  const [scoreImportFilePath, setScoreImportFilePath] = useState<string | null>(null);
  const [scorePreviewData, setScorePreviewData] = useState<{
    total_rows: number; valid_rows: number; error_rows: number;
    rows: Array<Record<string, unknown>>; errors: string[]; warnings: string[];
  } | null>(null);

  // ── forms ──
  const [examForm] = Form.useForm();
  const [scoreForm] = Form.useForm();
  const [subjectForm] = Form.useForm();
  const [configForm] = Form.useForm();

  // ── queries ──
  const { data: exams = [], isLoading: examsLoading } = useQuery({
    queryKey: ['exams', currentCohort?.id],
    queryFn: () => examService.list(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data: subjects = [] } = useQuery({
    queryKey: ['subjects'],
    queryFn: () => subjectService.list(),
  });

  const { data: students = [] } = useQuery({
    queryKey: ['allStudents', currentCohort?.id],
    queryFn: () => studentService.getAll(currentCohort!.id),
    enabled: !!currentCohort,
  });

  const { data: scores = [], isLoading: scoresLoading } = useQuery({
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

  // ── derived ──
  const selectedExam = useMemo(
    () => exams.find((e) => e.id === selectedExamId) ?? null,
    [exams, selectedExamId],
  );

  const activeSubjects = useMemo(() => {
    return examSubjectConfigs
      .filter((c) => (subjects.find((s) => s.id === c.subject_id)?.is_active ?? false)
        || c.subject_id === selectedSubjectId)
      .sort((a, b) => a.sort_order - b.sort_order);
  }, [examSubjectConfigs, subjects, selectedSubjectId]);

  const selectedSubject = useMemo(
    () => subjects.find((s) => s.id === selectedSubjectId) ?? null,
    [subjects, selectedSubjectId],
  );

  const selectedSubjectInactive = !!selectedSubject && !selectedSubject.is_active;

  const hasConfiguredSubjects = examSubjectConfigs.length > 0;

  // ── sync config form when modal opens ──
  useEffect(() => {
    if (!configModalVisible) return;
    const rows = (subjects || []).map((subject) => {
      const existing = examSubjectConfigs.find((c) => c.subject_id === subject.id);
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

  // ── clear subject if removed from config ──
  useEffect(() => {
    if (!selectedSubjectId) return;
    if (!examSubjectConfigs.some((c) => c.subject_id === selectedSubjectId)) {
      setSelectedSubjectId(null);
    }
  }, [examSubjectConfigs, selectedSubjectId]);

  // ── mutations (reuse existing logic) ──
  const createExamMutation = useMutation({
    mutationFn: (data: Partial<Exam>) => examService.create(data),
    onSuccess: (newExam) => {
      queryClient.invalidateQueries({ queryKey: ['exams'] });
      message.success('考试创建成功');
      setExamModalVisible(false);
      examForm.resetFields();
      // auto-select the new exam
      setSelectedExamId(newExam.id);
      setSelectedSubjectId(null);
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
      if (selectedExamId === id) {
        setSelectedExamId(null);
        setSelectedSubjectId(null);
      }
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
    mutationFn: (configs: ExamSubjectConfig[]) =>
      examService.saveSubjectConfigs(selectedExamId!, configs),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['examSubjectConfigs'] });
      queryClient.invalidateQueries({ queryKey: ['scoreStats'] });
      message.success('科目配置已保存');
      setConfigModalVisible(false);
    },
    onError: (err: Error) => message.error(err.message),
  });

  // ── event handlers ──
  const handleSelectExam = (examId: number) => {
    if (examId === selectedExamId) return;
    setSelectedExamId(examId);
    setSelectedSubjectId(null);
  };

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

  const handleDeleteExam = (id: number) => deleteExamMutation.mutate(id);

  const handleOpenConfig = (examId: number) => {
    if (examId !== selectedExamId) {
      setSelectedExamId(examId);
      // The config modal will populate from examSubjectConfigs
      // which re-fetches when selectedExamId changes.
      // We defer opening the modal in a useEffect so we have the right data.
    }
    setConfigModalVisible(true);
  };

  // Ensure handleOpenConfig also selects the exam if needed
  const handleConfigureCurrentExam = () => {
    if (!selectedExamId) return;
    setConfigModalVisible(true);
  };

  const handleSaveConfigs = async () => {
    const values = await configForm.validateFields();
    const configs = (values.configs || [])
      .filter((item: { enabled?: boolean }) => item.enabled)
      .map((item: ExamSubjectConfig) => {
        const full = Number(item.full_score);
        const pass = Number(item.pass_score);
        const excellent = Number(item.excellent_score);
        if (pass > full) {
          message.error(`科目配置错误：及格线(${pass})不能高于满分(${full})`);
          throw new Error('pass_score > full_score');
        }
        if (excellent > full) {
          message.error(`科目配置错误：优秀线(${excellent})不能高于满分(${full})`);
          throw new Error('excellent_score > full_score');
        }
        return {
          subject_id: item.subject_id,
          full_score: full,
          pass_score: pass,
          excellent_score: excellent,
          sort_order: Number(item.sort_order ?? 0),
        };
      });
    saveConfigMutation.mutate(configs);
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

  const handleDeleteSubject = (id: number) => deleteSubjectMutation.mutate(id);
  const handleToggleSubjectActive = (s: Subject) =>
    toggleSubjectActiveMutation.mutate({ id: s.id, is_active: !s.is_active });

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
        m.open({ multiple: false, filters: [{ name: 'Excel', extensions: ['xlsx', 'xls'] }] }),
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

  const handleConfirmImport = async () => {
    if (!selectedExamId || !selectedSubjectId || !scoreImportFilePath) return;
    try {
      const result = await scoreService.importExcel(selectedExamId, selectedSubjectId, scoreImportFilePath);
      if (result.errors.length > 0) {
        message.warning(`成功导入 ${result.success} 条，${result.errors.length} 条失败`);
      } else if (result.warnings.length > 0) {
        message.warning(`成功导入 ${result.success} 条，${result.warnings.length} 条覆盖提醒`);
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

  const handleDownloadTemplate = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: 'score_导入模板.xlsx',
      });
      if (!filePath) return;
      await configService.downloadTemplate('score', filePath);
      message.success('导入模板下载成功');
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '模板下载失败');
    }
  };

  const handleExportScores = async () => {
    if (!selectedExamId || !selectedSubjectId) return;
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: `${selectedExam?.name || '考试'}_${selectedSubject?.name || '科目'}_成绩.xlsx`,
      });
      if (!filePath) return;
      await scoreService.exportExcel(selectedExamId, selectedSubjectId, filePath);
      message.success('成绩导出成功');
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : '导出失败');
    }
  };

  // ── keyboard shortcuts ──
  useKeyboardShortcut('n', () => {
    if (isReadonly) return;
    setEditingExam(null);
    examForm.resetFields();
    setExamModalVisible(true);
  }, { ctrlOrMeta: true, enabled: !isReadonly });

  useKeyboardShortcut('s', handleSaveScores, {
    ctrlOrMeta: true,
    enabled: scoreModalVisible && !saveScoresMutation.isPending,
  });

  // ── score table data ──
  const scoreDataSource = students.map((s) => {
    const sc = scores.find((r) => r.student_id === s.id);
    return {
      key: s.id,
      student_name: s.name,
      student_no: s.student_no,
      score_value: sc?.score_value ?? null,
      rank_no: sc?.rank_no ?? null,
    };
  });

  // ── subject options for selector ──
  const subjectOptions = activeSubjects.map((c) => {
    const s = subjects.find((sub) => sub.id === c.subject_id);
    return {
      value: c.subject_id,
      label: `${s?.name ?? `科目 #${c.subject_id}`}（满分 ${c.full_score}）`,
    };
  });

  // ── render ──
  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Page Header */}
      <div className="page-header">
        <Space>
          <Title level={4} style={{ margin: 0 }}>成绩管理</Title>
          {selectedExam && (
            <>
              <Divider type="vertical" />
              <Space size={4}>
                <Text type="secondary" style={{ fontSize: 13 }}>正在管理：</Text>
                <Text strong style={{ fontSize: 13 }}>{selectedExam.name}</Text>
                {selectedExam.exam_type && <Tag>{selectedExam.exam_type}</Tag>}
              </Space>
            </>
          )}
        </Space>
        <Space>
          {!isReadonly && (
            <Button
              icon={<PlusOutlined />}
              onClick={() => {
                setEditingExam(null);
                examForm.resetFields();
                setExamModalVisible(true);
              }}
            >
              创建考试
            </Button>
          )}
          <Button
            icon={<PlusOutlined />}
            onClick={() => {
              setEditingSubject(null);
              subjectForm.resetFields();
              subjectForm.setFieldsValue({ is_active: true, sort_order: 0 });
              setSubjectModalVisible(true);
            }}
          >
            新增科目
          </Button>
        </Space>
      </div>

      {/* Main layout: sidebar + workbench */}
      <div style={{ flex: 1, display: 'flex', gap: 16, minHeight: 0 }}>
        {/* ── LEFT: Exam List Sidebar ── */}
        <Card
          size="small"
          styles={{
            body: { padding: 12 },
          }}
          style={{
            width: 300,
            flexShrink: 0,
            display: 'flex',
            flexDirection: 'column',
            overflow: 'hidden',
          }}
        >
          <div style={{
            display: 'flex', justifyContent: 'space-between', alignItems: 'center',
            marginBottom: 12, padding: '0 4px',
          }}>
            <Text strong style={{ fontSize: 13, color: textLabel }}>考试列表</Text>
            <Badge count={exams.length} size="small" showZero color="#1677ff" overflowCount={999} />
          </div>

          <div style={{ flex: 1, overflowY: 'auto', minHeight: 0 }}>
            {examsLoading ? (
              <Skeleton active paragraph={{ rows: 4 }} />
            ) : exams.length === 0 ? (
              <div style={{ textAlign: 'center', padding: '32px 16px' }}>
                <ExperimentOutlined style={{ fontSize: 32, color: iconMuted }} />
                <Paragraph type="secondary" style={{ marginTop: 12, fontSize: 13 }}>
                  还没有考试记录
                </Paragraph>
                {!isReadonly && (
                  <Button
                    type="primary"
                    size="small"
                    icon={<PlusOutlined />}
                    onClick={() => {
                      setEditingExam(null);
                      examForm.resetFields();
                      setExamModalVisible(true);
                    }}
                  >
                    创建第一场考试
                  </Button>
                )}
              </div>
            ) : (
              exams.map((exam) => (
                <ExamListItem
                  key={exam.id}
                  exam={exam}
                  isSelected={exam.id === selectedExamId}
                  onSelect={handleSelectExam}
                  onEdit={handleEditExam}
                  onDelete={handleDeleteExam}
                  onConfigure={handleOpenConfig}
                  isReadonly={isReadonly}
                  isDark={isDark}
                />
              ))
            )}
          </div>

          {/* Global subject pool */}
          <Divider style={{ margin: '12px 0 8px' }} />
          <div style={{ padding: '0 4px', marginBottom: 8, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span>
              <Text strong style={{ fontSize: 13, color: textLabel }}>科目池</Text>
              <Text type="secondary" style={{ fontSize: 11, marginLeft: 8 }}>全局可用科目</Text>
            </span>
            {!isReadonly && (
              <Button
                type="link"
                size="small"
                icon={<PlusOutlined />}
                onClick={() => {
                  setEditingSubject(null);
                  subjectForm.resetFields();
                  subjectForm.setFieldsValue({ is_active: true, sort_order: 0 });
                  setSubjectModalVisible(true);
                }}
                style={{ fontSize: 12, padding: 0 }}
              >
                新增
              </Button>
            )}
          </div>
          <div style={{ maxHeight: 160, overflowY: 'auto' }}>
            {subjects.length === 0 ? (
              <Text type="secondary" style={{ fontSize: 12, padding: '0 4px' }}>暂无科目</Text>
            ) : (
              subjects.slice(0, 10).map((s) => (
                <div key={s.id} style={{
                  display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                  padding: '4px 8px', fontSize: 12,
                }}>
                  <Space size={4}>
                    {s.is_active ? (
                      <Badge status="success" />
                    ) : (
                      <Badge status="default" />
                    )}
                    <Text style={{ fontSize: 12 }} delete={!s.is_active}>{s.name}</Text>
                  </Space>
                  {!isReadonly && (
                    <Space size={0}>
                      <Button
                        type="link"
                        size="small"
                        onClick={() => handleEditSubject(s)}
                        style={{ fontSize: 11, padding: '0 4px', height: 20 }}
                      >
                        编辑
                      </Button>
                      <Popconfirm
                        title={s.is_active ? '停用后历史数据会保留，确认停用？' : '确认重新启用该科目？'}
                        onConfirm={() => handleToggleSubjectActive(s)}
                      >
                        <Button
                          type="link"
                          size="small"
                          style={{ fontSize: 11, padding: '0 4px', height: 20 }}
                        >
                          {s.is_active ? '停用' : '启用'}
                        </Button>
                      </Popconfirm>
                      {!s.is_active && (
                        <Popconfirm
                          title="仅未被历史作业或成绩引用的停用科目才允许删除，确认删除？"
                          onConfirm={() => handleDeleteSubject(s.id)}
                        >
                          <Button
                            type="link"
                            size="small"
                            danger
                            style={{ fontSize: 11, padding: '0 4px', height: 20 }}
                          >
                            删除
                          </Button>
                        </Popconfirm>
                      )}
                    </Space>
                  )}
                </div>
              ))
            )}
          </div>
        </Card>

        {/* ── RIGHT: Exam Workbench ── */}
        <div style={{ flex: 1, minWidth: 0, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
          {!selectedExam ? (
            /* ── No exam selected: Empty State ── */
            <Card style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <div style={{ textAlign: 'center', padding: '48px 0', maxWidth: 400 }}>
                <TrophyOutlined style={{ fontSize: 56, color: iconMuted, marginBottom: 24 }} />
                <Title level={4} style={{ color: textMuted, marginBottom: 8 }}>
                  选择一场考试开始管理
                </Title>
                <Paragraph type="secondary" style={{ marginBottom: 24 }}>
                  在左侧列表中选择一场考试，即可进入该考试的工作台，<br />
                  进行科目配置、成绩录入和排名查看。
                </Paragraph>
                <Space direction="vertical" style={{ width: '100%' }} align="center">
                  {exams.length > 0 ? (
                    <Text type="secondary" style={{ fontSize: 13 }}>
                      <ArrowLeftOutlined style={{ marginRight: 4 }} />
                      点击左侧考试列表中的任意考试即可开始
                    </Text>
                  ) : (
                    !isReadonly && (
                      <Button
                        type="primary"
                        icon={<PlusOutlined />}
                        onClick={() => {
                          setEditingExam(null);
                          examForm.resetFields();
                          setExamModalVisible(true);
                        }}
                      >
                        创建第一场考试
                      </Button>
                    )
                  )}
                </Space>
              </div>
            </Card>
          ) : (
            /* ── Exam Workbench ── */
            <>
              {/* Section 1: Exam Info Banner */}
              <Card size="small" style={{ borderLeft: '3px solid #1677ff' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Space size={12}>
                    <TrophyOutlined style={{ fontSize: 22, color: '#1677ff' }} />
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <Title level={5} style={{ margin: 0 }}>{selectedExam.name}</Title>
                        {selectedExam.exam_type && <Tag color="blue">{selectedExam.exam_type}</Tag>}
                      </div>
                      <Space size={8} style={{ marginTop: 4 }}>
                        {selectedExam.exam_date ? (
                          <Text type="secondary" style={{ fontSize: 13 }}>
                            考试日期：{selectedExam.exam_date}
                          </Text>
                        ) : (
                          <Text type="secondary" style={{ fontSize: 13 }}>未设定日期</Text>
                        )}
                        {selectedExam.remark && (
                          <Text type="secondary" style={{ fontSize: 13 }}>备注：{selectedExam.remark}</Text>
                        )}
                        {hasConfiguredSubjects && (
                          <Tag color="green">{activeSubjects.length} 个科目的配置</Tag>
                        )}
                      </Space>
                    </div>
                  </Space>
                  {!isReadonly && (
                    <Space size={8}>
                      <Button
                        size="small"
                        icon={<EditOutlined />}
                        onClick={() => handleEditExam(selectedExam)}
                      >
                        编辑考试
                      </Button>
                      <Popconfirm
                        title="删除考试会同时删除该考试的所有成绩数据，确认删除？"
                        onConfirm={() => handleDeleteExam(selectedExam.id)}
                      >
                        <Button size="small" danger icon={<DeleteOutlined />}>删除</Button>
                      </Popconfirm>
                    </Space>
                  )}
                </div>
              </Card>

              {/* Section 2: Subject Configuration */}
              <Card
                size="small"
                title={
                  <Space>
                    <BookOutlined />
                    <span>科目配置与评分规则</span>
                    {hasConfiguredSubjects && (
                      <Tag color="processing">{activeSubjects.length} 个科目</Tag>
                    )}
                  </Space>
                }
                extra={
                  !isReadonly && (
                    <Button
                      type="link"
                      size="small"
                      icon={<SettingOutlined />}
                      onClick={handleConfigureCurrentExam}
                    >
                      {hasConfiguredSubjects ? '管理科目配置' : '配置考试科目'}
                    </Button>
                  )
                }
              >
                {!hasConfiguredSubjects ? (
                  <div style={{
                    textAlign: 'center', padding: '24px 0',
                    background: panelBg, borderRadius: 8,
                  }}>
                    <ExclamationCircleOutlined style={{ fontSize: 28, color: '#faad14', marginBottom: 12 }} />
                    <Paragraph type="secondary" style={{ marginBottom: 16 }}>
                      当前考试尚未配置科目
                    </Paragraph>
                    {!isReadonly && (
                      <Button
                        type="primary"
                        icon={<SettingOutlined />}
                        onClick={handleConfigureCurrentExam}
                      >
                        立即配置考试科目
                      </Button>
                    )}
                  </div>
                ) : (
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
                    {activeSubjects.map((c) => {
                      const sub = subjects.find((s) => s.id === c.subject_id);
                      const isCurrentSubject = c.subject_id === selectedSubjectId;
                      return (
                        <div
                          key={c.subject_id}
                          onClick={() => {
                            setSelectedSubjectId(
                              isCurrentSubject ? null : c.subject_id,
                            );
                          }}
                          style={{
                            padding: '10px 16px',
                            borderRadius: 8,
                            border: isCurrentSubject
                              ? '2px solid #1677ff'
                              : `1px solid ${itemBorder}`,
                            background: isCurrentSubject ? itemSelectedBg : itemBg,
                            cursor: 'pointer',
                            minWidth: 160,
                            transition: 'all 0.2s',
                            userSelect: 'none',
                          }}
                        >
                          <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 4 }}>
                            {isCurrentSubject && (
                              <CheckCircleOutlined
                                style={{ color: '#1677ff', marginRight: 4 }}
                              />
                            )}
                            {sub?.name ?? `科目 #${c.subject_id}`}
                          </div>
                          <Text type="secondary" style={{ fontSize: 11 }}>
                            满分 {c.full_score} · 及格 {c.pass_score} · 优秀 {c.excellent_score}
                          </Text>
                        </div>
                      );
                    })}
                  </div>
                )}
              </Card>

              {/* Section 3: Score Entry */}
              {hasConfiguredSubjects && (
                <Card
                  size="small"
                  title={
                    <Space>
                      <FileTextOutlined />
                      <span>成绩录入与操作</span>
                      {selectedSubject && (
                        <>
                          <Tag color="blue">{selectedSubject.name}</Tag>
                          {selectedSubjectInactive && (
                            <Tag color="orange">已停用（仅可查看导出）</Tag>
                          )}
                        </>
                      )}
                    </Space>
                  }
                  extra={
                    <Space size={8}>
                      {selectedSubjectId && (
                        <Button
                          size="small"
                          icon={<DownloadOutlined />}
                          onClick={handleExportScores}
                        >
                          导出成绩
                        </Button>
                      )}
                      {!isReadonly && selectedSubjectId && !selectedSubjectInactive && (
                        <>
                          <Button
                            size="small"
                            type="primary"
                            onClick={() => {
                              scoreForm.resetFields();
                              const iv: Record<string, number | null> = {};
                              students.forEach((s) => {
                                const sc = scores.find(
                                  (r) => r.student_id === s.id,
                                );
                                iv[`score_${s.id}`] = sc?.score_value ?? null;
                              });
                              scoreForm.setFieldsValue(iv);
                              setScoreModalVisible(true);
                            }}
                          >
                            录入成绩
                          </Button>
                          <Button
                            size="small"
                            icon={<UploadOutlined />}
                            onClick={handleImportScores}
                          >
                            导入
                          </Button>
                          <Button
                            size="small"
                            icon={<DownloadOutlined />}
                            onClick={handleDownloadTemplate}
                          >
                            下载模板
                          </Button>
                        </>
                      )}
                    </Space>
                  }
                >
                  {/* Subject Selector */}
                  <div style={{ marginBottom: 16 }}>
                    <Select
                      style={{ width: 280 }}
                      placeholder="选择要操作的科目"
                      value={selectedSubjectId}
                      onChange={setSelectedSubjectId}
                      options={subjectOptions}
                      allowClear
                    />
                  </div>

                  {selectedSubjectId ? (
                    <>
                      {/* Statistics */}
                      {scoreStats && (
                        <div
                          style={{
                            marginBottom: 16,
                            padding: '12px 16px',
                            background: panelBg,
                            borderRadius: 8,
                          }}
                        >
                          <Text strong style={{ fontSize: 12, color: textMuted, display: 'block', marginBottom: 8 }}>
                            成绩统计
                          </Text>
                          <Row gutter={[16, 8]}>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 22, fontWeight: 700, color: '#1677ff' }}>
                                  {scoreStats.avg_score.toFixed(1)}
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>平均分</div>
                              </div>
                            </Col>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 22, fontWeight: 700, color: '#52c41a' }}>
                                  {scoreStats.max_score}
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>最高分</div>
                              </div>
                            </Col>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 22, fontWeight: 700, color: '#ff4d4f' }}>
                                  {scoreStats.min_score}
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>最低分</div>
                              </div>
                            </Col>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 22, fontWeight: 700, color: '#722ed1' }}>
                                  {(scoreStats.pass_rate * 100).toFixed(0)}%
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>及格率</div>
                              </div>
                            </Col>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 22, fontWeight: 700, color: '#fa8c16' }}>
                                  {(scoreStats.excellent_rate * 100).toFixed(0)}%
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>优秀率</div>
                              </div>
                            </Col>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 16, fontWeight: 600, color: textLabel }}>
                                  {scoreStats.full_score}
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>满分</div>
                              </div>
                            </Col>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 16, fontWeight: 600, color: textLabel }}>
                                  {scoreStats.pass_score}
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>及格线</div>
                              </div>
                            </Col>
                            <Col span={6}>
                              <div style={{ textAlign: 'center' }}>
                                <div style={{ fontSize: 16, fontWeight: 600, color: textLabel }}>
                                  {scoreStats.excellent_score}
                                </div>
                                <div style={{ fontSize: 11, color: textMuted }}>优秀线</div>
                              </div>
                            </Col>
                          </Row>
                        </div>
                      )}

                      {/* Score Table */}
                      <Table
                        dataSource={scoreDataSource}
                        columns={[
                          { title: '姓名', dataIndex: 'student_name', key: 'student_name', width: 120 },
                          { title: '学号', dataIndex: 'student_no', key: 'student_no', width: 120 },
                          {
                            title: '成绩',
                            dataIndex: 'score_value',
                            key: 'score_value',
                            width: 100,
                            render: (v: number | null) =>
                              v !== null ? <Text strong>{v}</Text> : <Text type="secondary">-</Text>,
                          },
                          {
                            title: '排名',
                            dataIndex: 'rank_no',
                            key: 'rank_no',
                            width: 80,
                            render: (v: number | null) =>
                              v ? <Tag color="blue">{v}</Tag> : '-',
                          },
                        ]}
                        rowKey="key"
                        loading={scoresLoading}
                        size="small"
                        pagination={students.length > 20 ? { pageSize: 20, showSizeChanger: false } : false}
                        locale={{ emptyText: '暂无成绩记录' }}
                      />
                    </>
                  ) : (
                    <div style={{
                      textAlign: 'center', padding: '24px 0',
                      background: panelBg, borderRadius: 8,
                    }}>
                      <Text type="secondary">请在上方选择一个科目，即可查看和录入成绩</Text>
                    </div>
                  )}
                </Card>
              )}

              {/* Section 4: Rankings */}
              <Card
                size="small"
                title={
                  <Space>
                    <BarChartOutlined />
                    <span>总分排名</span>
                    {rankings && rankings.length > 0 && (
                      <Tag color="blue">{rankings.length} 人</Tag>
                    )}
                  </Space>
                }
              >
                {rankings && rankings.length > 0 ? (
                  <Table
                    dataSource={rankings.map((r) => ({ ...r, key: r.student_id }))}
                    columns={[
                      {
                        title: '排名',
                        dataIndex: 'rank_no',
                        key: 'rank_no',
                        width: 80,
                        render: (v: number) => {
                          if (v === 1) return <Tag color="gold">🥇 {v}</Tag>;
                          if (v === 2) return <Tag color="default">🥈 {v}</Tag>;
                          if (v === 3) return <Tag color="orange">🥉 {v}</Tag>;
                          return <Tag color="blue">{v}</Tag>;
                        },
                      },
                      { title: '姓名', dataIndex: 'student_name', key: 'student_name' },
                      { title: '学号', dataIndex: 'student_no', key: 'student_no' },
                      {
                        title: '总分',
                        dataIndex: 'total_score',
                        key: 'total_score',
                        render: (v: number) => <Text strong>{v.toFixed(1)}</Text>,
                      },
                    ]}
                    rowKey="key"
                    size="small"
                    pagination={rankings.length > 20 ? { pageSize: 20, showSizeChanger: false } : false}
                  />
                ) : (
                  <div style={{
                    textAlign: 'center', padding: '20px 0',
                    background: panelBg, borderRadius: 8,
                  }}>
                    <Text type="secondary">
                      {hasConfiguredSubjects
                        ? '暂无排名数据，请先录入各科目成绩'
                        : '请先配置考试科目并录入成绩'}
                    </Text>
                  </div>
                )}
              </Card>
            </>
          )}
        </div>
      </div>

      {/* ─── Modals (unchanged business logic) ─── */}

      {/* Exam CRUD Modal */}
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
                <Select
                  allowClear
                  options={[
                    { value: '月考', label: '月考' },
                    { value: '期中', label: '期中' },
                    { value: '期末', label: '期末' },
                    { value: '模拟', label: '模拟' },
                  ]}
                />
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

      {/* Subject CRUD Modal */}
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

      {/* Subject Config Modal */}
      <Modal
        title="配置考试科目与评分规则"
        open={configModalVisible}
        onOk={handleSaveConfigs}
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
                      <Form.Item name={[field.name, 'full_score']} rules={[{ required: true, message: '请输入满分' }]} noStyle>
                        <Input type="number" min={1} />
                      </Form.Item>
                    ),
                  },
                  {
                    title: '及格线',
                    render: (_, field) => (
                      <Form.Item name={[field.name, 'pass_score']} rules={[{ required: true, message: '请输入及格线' }]} noStyle>
                        <Input type="number" min={0} />
                      </Form.Item>
                    ),
                  },
                  {
                    title: '优秀线',
                    render: (_, field) => (
                      <Form.Item name={[field.name, 'excellent_score']} rules={[{ required: true, message: '请输入优秀线' }]} noStyle>
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

      {/* Score Entry Modal */}
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
            dataSource={students.map((s) => ({
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
            pagination={students.length > 20 ? { pageSize: 20 } : false}
            size="small"
          />
        </Form>
      </Modal>

      {/* Score Import Preview Modal */}
      <Modal
        title="成绩导入预览"
        open={scorePreviewVisible}
        onCancel={() => {
          setScorePreviewVisible(false);
          setScorePreviewData(null);
          setScoreImportFilePath(null);
        }}
        onOk={handleConfirmImport}
        okButtonProps={{
          disabled:
            !scorePreviewData ||
            scorePreviewData.valid_rows === 0 ||
            scorePreviewData.error_rows > 0,
        }}
        width={960}
      >
        {scorePreviewData && (
          <>
            <Space style={{ marginBottom: 12 }}>
              <Tag color="blue">共 {scorePreviewData.total_rows} 条</Tag>
              <Tag color="green">有效 {scorePreviewData.valid_rows} 条</Tag>
              {scorePreviewData.error_rows > 0 && (
                <Tag color="red">错误 {scorePreviewData.error_rows} 条</Tag>
              )}
              {scorePreviewData.warnings.length > 0 && (
                <Tag color="orange">覆盖提醒 {scorePreviewData.warnings.length} 条</Tag>
              )}
            </Space>
            {scorePreviewData.errors.length > 0 && (
              <Alert
                type="error"
                showIcon
                style={{ marginBottom: 12 }}
                message="以下错误会阻止导入，请先修正文件："
                description={
                  <ul style={{ margin: 0, paddingLeft: 16 }}>
                    {scorePreviewData.errors.map((e, i) => <li key={i}>{e}</li>)}
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
                { title: '姓名', dataIndex: 'student_name', width: 120, render: (v: string | null) => v || '-' },
                { title: '成绩', dataIndex: 'score_value', width: 100, render: (v: number | null) => v ?? '-' },
                {
                  title: '状态',
                  dataIndex: 'valid',
                  width: 100,
                  render: (v: boolean) => <Tag color={v ? 'green' : 'red'}>{v ? '有效' : '错误'}</Tag>,
                },
                { title: '提醒', dataIndex: 'warning', render: (v: string | null) => v || '-' },
              ]}
            />
          </>
        )}
      </Modal>
    </div>
  );
}

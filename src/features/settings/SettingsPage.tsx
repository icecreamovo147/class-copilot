import { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Card,
  Descriptions,
  Divider,
  Form,
  Input,
  InputNumber,
  List,
  message,
  Modal,
  Radio,
  Select,
  Space,
  Tag,
  Typography,
} from 'antd';
import {
  ExportOutlined,
  FileExcelOutlined,
  FolderOpenOutlined,
  ReloadOutlined,
  SaveOutlined,
} from '@ant-design/icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { backupService, configService, getCommandErrorMessage } from '@/services';
import { invoke } from '@tauri-apps/api/core';

const { Title, Text, Paragraph } = Typography;

export default function SettingsPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { currentCohort, cohorts } = useAppStore();
  const [form] = Form.useForm();
  const [restoreModalVisible, setRestoreModalVisible] = useState(false);
  const [exportModalVisible, setExportModalVisible] = useState(false);
  const [exportCohortId, setExportCohortId] = useState<number | null>(null);
  const [exporting, setExporting] = useState(false);
  const [restoreFilePath, setRestoreFilePath] = useState<string | null>(null);
  const [logDir, setLogDir] = useState<string>('');

  const { data: overview, isLoading: overviewLoading } = useQuery({
    queryKey: ['settingsOverview'],
    queryFn: () => configService.getOverview(),
  });

  useEffect(() => {
    invoke<string>('get_log_dir').then(setLogDir).catch(() => setLogDir('不可用'));
  }, []);

  useEffect(() => {
    if (!overview) return;
    form.setFieldsValue({
      school_name: overview.school_name || '',
      head_teacher: overview.head_teacher || '',
      default_semester: overview.default_semester || '',
      default_backup_dir: overview.default_backup_dir,
      reminder_threshold: overview.reminder_threshold,
      export_preference: overview.export_preference,
    });
  }, [overview, form]);

  const backupDefaultPath = useMemo(() => {
    if (!overview?.default_backup_dir) {
      return `班级管理系统备份_${new Date().toISOString().slice(0, 10)}.bak`;
    }
    return `${overview.default_backup_dir}/班级管理系统备份_${new Date().toISOString().slice(0, 10)}.bak`;
  }, [overview]);

  const preferencesMutation = useMutation({
    mutationFn: (values: Record<string, unknown>) => configService.savePreferences(values as never),
    onSuccess: async () => {
      message.success('设置已保存');
      await queryClient.invalidateQueries({ queryKey: ['settingsOverview'] });
    },
    onError: (error) => message.error(getCommandErrorMessage(error)),
  });

  const backupMutation = useMutation({
    mutationFn: (filePath: string) => backupService.create(filePath),
    onSuccess: async () => {
      message.success('备份成功');
      await queryClient.invalidateQueries({ queryKey: ['settingsOverview'] });
    },
    onError: (error) => message.error(getCommandErrorMessage(error)),
  });

  const restoreMutation = useMutation({
    mutationFn: (filePath: string) => backupService.restore(filePath),
    onSuccess: () => {
      message.success('恢复成功，正在刷新...');
      setRestoreModalVisible(false);
      setRestoreFilePath(null);
      setTimeout(() => window.location.reload(), 2000);
    },
    onError: (error) => message.error(getCommandErrorMessage(error)),
  });

  const handleOpenLogDir = async () => {
    try {
      await invoke('open_log_dir');
      message.success('已在文件管理器中打开日志目录');
    } catch {
      message.error('无法打开日志目录');
    }
  };

  const handlePickBackupDir = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        form.setFieldValue('default_backup_dir', selected);
      }
    } catch {
      message.error('选择目录失败');
    }
  };

  const handleSavePreferences = async () => {
    const values = await form.validateFields();
    preferencesMutation.mutate(values);
  };

  const handleBackup = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: '备份文件', extensions: ['bak'] }],
        defaultPath: backupDefaultPath,
      });
      if (filePath) {
        backupMutation.mutate(filePath);
      }
    } catch {
      message.error('备份操作失败');
    }
  };

  const handleRestore = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const filePath = await open({
        filters: [{ name: '备份文件', extensions: ['bak'] }],
        multiple: false,
      });
      if (filePath) {
        setRestoreFilePath(filePath as string);
        setRestoreModalVisible(true);
      }
    } catch {
      message.error('恢复操作失败');
    }
  };

  const handleRestoreFromRecent = (filePath: string) => {
    setRestoreFilePath(filePath);
    setRestoreModalVisible(true);
  };

  const handleExportCohort = async () => {
    if (!exportCohortId) {
      message.warning('请选择要导出的届次');
      return;
    }
    setExporting(true);
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const targetCohort = cohorts.find((c) => c.id === exportCohortId);
      const filePath = await save({
        filters: [{ name: 'ZIP 文件', extensions: ['zip'] }],
        defaultPath: `导出_${targetCohort?.cohort_name}_${targetCohort?.class_name}.zip`,
      });
      if (filePath) {
        await backupService.exportCohort(exportCohortId, filePath);
        message.success('导出成功');
        setExportModalVisible(false);
      }
    } catch {
      message.error('导出失败');
    } finally {
      setExporting(false);
    }
  };

  const handleDownloadTemplate = async (type: string) => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
        defaultPath: `${type}_导入模板.xlsx`,
      });
      if (filePath) {
        await configService.downloadTemplate(type, filePath);
        message.success('模板下载成功');
      }
    } catch {
      message.error('模板下载失败');
    }
  };

  return (
    <div>
      <Title level={4}>系统设置</Title>

      <Card title="默认偏好" loading={overviewLoading} style={{ marginBottom: 16 }}>
        <Form form={form} layout="vertical">
          <Space direction="vertical" style={{ width: '100%' }} size={0}>
            <Form.Item name="school_name" label="学校名称">
              <Input placeholder="用于新建届次时预填" />
            </Form.Item>
            <Form.Item name="head_teacher" label="班主任">
              <Input placeholder="用于新建届次时预填" />
            </Form.Item>
            <Form.Item name="default_semester" label="默认学期">
              <Input placeholder="如：2026 春季学期" />
            </Form.Item>
            <Form.Item name="default_backup_dir" label="默认备份目录" rules={[{ required: true, message: '请选择默认备份目录' }]}>
              <Input
                addonAfter={
                  <Button type="link" onClick={handlePickBackupDir} style={{ paddingInline: 0 }}>
                    选择
                  </Button>
                }
              />
            </Form.Item>
            <Form.Item name="reminder_threshold" label="提醒阈值（天/次）" rules={[{ required: true, message: '请输入提醒阈值' }]}>
              <InputNumber min={1} max={30} style={{ width: 180 }} />
            </Form.Item>
            <Form.Item name="export_preference" label="导出偏好" rules={[{ required: true, message: '请选择导出偏好' }]}>
              <Radio.Group
                options={[
                  { label: '仅 Excel', value: 'xlsx' },
                  { label: '仅 PDF', value: 'pdf' },
                  { label: 'Excel + PDF', value: 'both' },
                ]}
              />
            </Form.Item>
            <Button type="primary" onClick={handleSavePreferences} loading={preferencesMutation.isPending}>
              保存偏好
            </Button>
          </Space>
        </Form>
      </Card>

      <Card title="当前届次" style={{ marginBottom: 16 }}>
        {currentCohort ? (
          <Descriptions column={2} size="small" bordered>
            <Descriptions.Item label="届次名称">{currentCohort.cohort_name}</Descriptions.Item>
            <Descriptions.Item label="班级名称">{currentCohort.class_name}</Descriptions.Item>
            <Descriptions.Item label="状态">{currentCohort.status}</Descriptions.Item>
            <Descriptions.Item label="班主任">{currentCohort.head_teacher || '-'}</Descriptions.Item>
            <Descriptions.Item label="学校">{currentCohort.school_name || '-'}</Descriptions.Item>
            <Descriptions.Item label="学期">{currentCohort.semester || '-'}</Descriptions.Item>
          </Descriptions>
        ) : (
          <Paragraph type="secondary">尚未设置当前届次</Paragraph>
        )}
        <Button style={{ marginTop: 8 }} onClick={() => navigate('/cohorts')}>
          前往届次管理
        </Button>
      </Card>

      <Card title="数据备份与恢复" style={{ marginBottom: 16 }}>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Button type="primary" icon={<SaveOutlined />} onClick={handleBackup} loading={backupMutation.isPending}>
            全量备份
          </Button>
          <Paragraph type="secondary" style={{ margin: 0 }}>
            备份前会执行数据库一致性检查，并生成单个自包含备份包，包含数据库文件与校验清单。
          </Paragraph>
          <Divider />
          <Button icon={<ReloadOutlined />} onClick={handleRestore} loading={restoreMutation.isPending}>
            恢复数据
          </Button>
          <Paragraph type="secondary" style={{ margin: 0 }}>
            恢复前系统会自动备份当前数据库；校验不通过的备份文件会被直接拒绝。
          </Paragraph>
          <Divider />
          <Text strong>最近备份</Text>
          <List
            size="small"
            locale={{ emptyText: '默认备份目录下暂无备份文件' }}
            dataSource={overview?.recent_backups || []}
            renderItem={(item) => (
              <List.Item
                actions={[
                  <Button key="restore" type="link" onClick={() => handleRestoreFromRecent(item.file_path)}>
                    恢复
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={item.file_name}
                  description={`${item.modified_at} · ${(item.size_bytes / 1024 / 1024).toFixed(2)} MB`}
                />
              </List.Item>
            )}
          />
        </Space>
      </Card>

      <Card title="按届次导出" style={{ marginBottom: 16 }}>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Button icon={<ExportOutlined />} onClick={() => setExportModalVisible(true)}>
            导出届次数据
          </Button>
          <Paragraph type="secondary" style={{ margin: 0 }}>
            将指定届次的学生、作业、考勤、成绩和事务数据导出为 ZIP 文件。
          </Paragraph>
        </Space>
      </Card>

      <Card title="导入模板下载" style={{ marginBottom: 16 }}>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Space>
            <Button icon={<FileExcelOutlined />} onClick={() => handleDownloadTemplate('student')}>
              学生导入模板
            </Button>
            <Button icon={<FileExcelOutlined />} onClick={() => handleDownloadTemplate('score')}>
              成绩导入模板
            </Button>
          </Space>
        </Space>
      </Card>

      <Card title="系统日志" style={{ marginBottom: 16 }}>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Button icon={<FolderOpenOutlined />} onClick={handleOpenLogDir}>
            打开日志目录
          </Button>
          {logDir && (
            <Paragraph type="secondary" style={{ margin: 0, wordBreak: 'break-all' }}>
              日志路径：{logDir}
            </Paragraph>
          )}
        </Space>
      </Card>

      <Card title="关于系统" loading={overviewLoading}>
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="应用名称">数字化班级事务管理系统</Descriptions.Item>
          <Descriptions.Item label="应用版本">{overview?.app_version || '-'}</Descriptions.Item>
          <Descriptions.Item label="数据库版本">{overview?.database_version || '-'}</Descriptions.Item>
          <Descriptions.Item label="数据目录">{overview?.data_dir || '-'}</Descriptions.Item>
          <Descriptions.Item label="数据库文件">{overview?.database_path || '-'}</Descriptions.Item>
          <Descriptions.Item label="技术栈">Tauri 2.x + React + Rust + SQLite</Descriptions.Item>
          <Descriptions.Item label="导出偏好">
            <Tag>{overview?.export_preference || '-'}</Tag>
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Modal
        title="按届次导出"
        open={exportModalVisible}
        onOk={handleExportCohort}
        onCancel={() => setExportModalVisible(false)}
        confirmLoading={exporting}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Text>选择要导出的届次：</Text>
          <Select
            style={{ width: '100%' }}
            placeholder="选择届次"
            value={exportCohortId}
            onChange={setExportCohortId}
            options={cohorts.map((c) => ({
              value: c.id,
              label: `${c.cohort_name} ${c.class_name}`,
            }))}
          />
        </Space>
      </Modal>

      <Modal
        title="数据恢复确认"
        open={restoreModalVisible}
        onOk={() => restoreFilePath && restoreMutation.mutate(restoreFilePath)}
        onCancel={() => !restoreMutation.isPending && setRestoreModalVisible(false)}
        okText="确认恢复"
        okButtonProps={{ danger: true }}
        confirmLoading={restoreMutation.isPending}
      >
        <Paragraph>恢复将覆盖当前数据，并在恢复前自动生成一份恢复前备份。</Paragraph>
        <Paragraph type="secondary" style={{ marginBottom: 0, wordBreak: 'break-all' }}>
          待恢复文件：{restoreFilePath || '未选择'}
        </Paragraph>
      </Modal>
    </div>
  );
}

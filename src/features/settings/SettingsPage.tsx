import { useState } from 'react';
import { Button, Card, Descriptions, Divider, message, Modal, Select, Space, Typography } from 'antd';
import { ExportOutlined, FileExcelOutlined, ReloadOutlined, SaveOutlined } from '@ant-design/icons';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '@/app/store';
import { backupService, configService, getCommandErrorMessage } from '@/services';

const { Title, Text, Paragraph } = Typography;

export default function SettingsPage() {
  const navigate = useNavigate();
  const { currentCohort, cohorts, appVersion } = useAppStore();
  const [restoreModalVisible, setRestoreModalVisible] = useState(false);
  const [exportModalVisible, setExportModalVisible] = useState(false);
  const [exportCohortId, setExportCohortId] = useState<number | null>(null);
  const [exporting, setExporting] = useState(false);
  const [restoreFilePath, setRestoreFilePath] = useState<string | null>(null);

  const backupMutation = useMutation({
    mutationFn: (filePath: string) => backupService.create(filePath),
    onSuccess: () => message.success('备份成功'),
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

  const handleBackup = async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        filters: [{ name: '备份文件', extensions: ['bak'] }],
        defaultPath: `班级管理系统备份_${new Date().toISOString().slice(0, 10)}.bak`,
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

  const handleRestoreConfirm = () => {
    if (restoreFilePath) {
      restoreMutation.mutate(restoreFilePath);
    }
  };

  const handleRestoreCancel = () => {
    if (!restoreMutation.isPending) {
      setRestoreModalVisible(false);
      setRestoreFilePath(null);
    }
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
            将当前所有数据保存到备份文件。建议定期备份以防数据丢失。
          </Paragraph>
          <Divider />
          <Button icon={<ReloadOutlined />} onClick={handleRestore} loading={restoreMutation.isPending}>
            恢复数据
          </Button>
          <Paragraph type="secondary" style={{ margin: 0 }}>
            从备份文件恢复数据。恢复前系统将自动备份当前数据库。
          </Paragraph>
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
          <Paragraph type="secondary" style={{ margin: 0 }}>
            下载标准 Excel 导入模板，请按照模板格式准备导入文件。
          </Paragraph>
        </Space>
      </Card>

      <Card title="关于系统">
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="应用名称">数字化班级事务管理系统</Descriptions.Item>
          <Descriptions.Item label="版本号">{appVersion}</Descriptions.Item>
          <Descriptions.Item label="技术栈">Tauri 2.x + React + Rust + SQLite</Descriptions.Item>
          <Descriptions.Item label="支持平台">Windows / macOS</Descriptions.Item>
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
        onOk={handleRestoreConfirm}
        onCancel={handleRestoreCancel}
        okText="确认恢复"
        okButtonProps={{ danger: true }}
        confirmLoading={restoreMutation.isPending}
        cancelButtonProps={{ disabled: restoreMutation.isPending }}
      >
        <Paragraph>
          <strong>警告：</strong>数据恢复将覆盖当前所有数据。
        </Paragraph>
        <Paragraph>
          恢复前系统会自动备份当前数据库。确认要恢复吗？
        </Paragraph>
      </Modal>
    </div>
  );
}


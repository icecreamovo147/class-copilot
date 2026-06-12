import { useEffect, useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, Select, Tag, Button, Typography, Space, message } from 'antd';
import {
  DashboardOutlined,
  TeamOutlined,
  BookOutlined,
  CalendarOutlined,
  TrophyOutlined,
  AppstoreOutlined,
  BarChartOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  ApartmentOutlined,
} from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import { cohortService } from '@/services';

const { Header, Sider, Content } = Layout;
const { Text } = Typography;

const menuItems = [
  { key: '/dashboard', icon: <DashboardOutlined />, label: '首页看板' },
  { key: '/cohorts', icon: <ApartmentOutlined />, label: '届次管理' },
  { key: '/students', icon: <TeamOutlined />, label: '学生信息' },
  { key: '/homework', icon: <BookOutlined />, label: '作业管理' },
  { key: '/attendance', icon: <CalendarOutlined />, label: '考勤请假' },
  { key: '/scores', icon: <TrophyOutlined />, label: '成绩管理' },
  { key: '/affairs', icon: <AppstoreOutlined />, label: '班级事务' },
  { key: '/statistics', icon: <BarChartOutlined />, label: '数据统计' },
  { key: '/settings', icon: <SettingOutlined />, label: '系统设置' },
];

export default function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { currentCohort, setCurrentCohort, cohorts, setCohorts, sidebarCollapsed, toggleSidebar, isReadonly } = useAppStore();
  const [initializing, setInitializing] = useState(true);

  const { data: cohortsData } = useQuery({
    queryKey: ['cohorts'],
    queryFn: () => cohortService.list(),
  });

  const { data: currentData } = useQuery({
    queryKey: ['currentCohort'],
    queryFn: () => cohortService.getCurrent(),
  });

  useEffect(() => {
    if (cohortsData) {
      setCohorts(cohortsData);
    }
  }, [cohortsData, setCohorts]);

  useEffect(() => {
    if (currentData) {
      setCurrentCohort(currentData);
      setInitializing(false);
    } else if (cohortsData && cohortsData.length === 0) {
      setInitializing(false);
    }
  }, [currentData, cohortsData, setCurrentCohort]);

  useEffect(() => {
    if (!currentData && cohortsData) {
      setInitializing(false);
    }
  }, [currentData, cohortsData]);

  const handleCohortChange = async (cohortId: number) => {
    try {
      await cohortService.setCurrent(cohortId);
      const cohort = cohorts.find((c) => c.id === cohortId);
      if (cohort) {
        setCurrentCohort(cohort);
      }
      message.success(`已切换到届次`);
      // 刷新所有缓存
      window.location.reload();
    } catch {
      message.error('切换届次失败');
    }
  };

  const handleMenuClick = (info: { key: string }) => {
    // 如果没有当前届次，只允许访问届次管理和设置
    if (!currentCohort && info.key !== '/cohorts' && info.key !== '/settings') {
      message.warning('请先创建或切换到一个当前届次');
      navigate('/cohorts');
      return;
    }
    navigate(info.key);
  };

  return (
    <Layout style={{ height: '100vh', overflow: 'hidden' }}>
      <Sider
        trigger={null}
        collapsible
        collapsed={sidebarCollapsed}
        theme="light"
        style={{
          borderRight: '1px solid #f0f0f0',
          overflow: 'auto',
          height: '100vh',
        }}
      >
        <div
          style={{
            height: 64,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderBottom: '1px solid #f0f0f0',
          }}
        >
          <Typography.Title level={5} style={{ margin: 0, whiteSpace: 'nowrap' }}>
            {sidebarCollapsed ? '班管' : '班级事务管理系统'}
          </Typography.Title>
        </div>
        <Menu
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={handleMenuClick}
        />
      </Sider>
      <Layout style={{ minWidth: 0, height: '100vh', overflow: 'hidden' }}>
        <Header
          style={{
            background: '#fff',
            padding: '0 24px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            borderBottom: '1px solid #f0f0f0',
            height: 64,
            flex: '0 0 64px',
          }}
        >
          <Space>
            <Button
              type="text"
              icon={sidebarCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
              onClick={toggleSidebar}
            />
            {!sidebarCollapsed && currentCohort && (
              <Space>
                <Text strong>
                  {currentCohort.cohort_name} {currentCohort.class_name}
                </Text>
                {isReadonly && <Tag color="default">已归档</Tag>}
              </Space>
            )}
          </Space>
          <Space>
            <Text type="secondary">当前届次：</Text>
            <Select
              style={{ minWidth: 160 }}
              placeholder="选择届次"
              value={currentCohort?.id ?? undefined}
              onChange={handleCohortChange}
              options={cohorts.map((c) => ({
                value: c.id,
                label: `${c.cohort_name} ${c.class_name} ${c.status === '已归档' ? '(已归档)' : ''}`,
              }))}
              loading={cohorts.length === 0 && initializing}
            />
          </Space>
        </Header>
        <Content style={{ margin: 24, minHeight: 0, overflowY: 'auto', overflowX: 'hidden' }}>
          {!currentCohort && !initializing && location.pathname !== '/cohorts' && location.pathname !== '/settings' ? (
            <div style={{ textAlign: 'center', padding: '80px 0' }}>
              <Typography.Title level={4}>尚未创建届次</Typography.Title>
              <Typography.Paragraph type="secondary">
                请先进入届次管理创建第一个届次档案
              </Typography.Paragraph>
              <Button type="primary" onClick={() => navigate('/cohorts')}>
                前往届次管理
              </Button>
            </div>
          ) : (
            <div className="page-transition" key={location.pathname}>
              <Outlet />
            </div>
          )}
        </Content>
      </Layout>
    </Layout>
  );
}

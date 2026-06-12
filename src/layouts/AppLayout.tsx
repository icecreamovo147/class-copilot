import { useEffect, useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, Select, Tag, Button, Typography, Space, message, Dropdown } from 'antd';
import type { MenuProps } from 'antd';
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
  SunOutlined,
  MoonOutlined,
  DesktopOutlined,
} from '@ant-design/icons';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '@/app/store';
import type { ThemeMode } from '@/app/store';
import { cohortService } from '@/services';
import { useIsDark } from '@/hooks/useTheme';

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

const themeMenuItems: MenuProps['items'] = [
  {
    key: 'light',
    icon: <SunOutlined />,
    label: '浅色模式',
  },
  {
    key: 'dark',
    icon: <MoonOutlined />,
    label: '深色模式',
  },
  {
    key: 'auto',
    icon: <DesktopOutlined />,
    label: '跟随系统',
  },
];

const themeIconMap: Record<ThemeMode, React.ReactNode> = {
  light: <SunOutlined />,
  dark: <MoonOutlined />,
  auto: <DesktopOutlined />,
};

export default function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const {
    currentCohort, setCurrentCohort,
    cohorts, setCohorts,
    sidebarCollapsed, toggleSidebar,
    isReadonly,
    themeMode, setThemeMode,
  } = useAppStore();
  const queryClient = useQueryClient();
  const [initializing, setInitializing] = useState(true);
  const isDark = useIsDark();

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
    } else if (cohortsData !== undefined) {
      // currentData 确定为空且 cohortsData 已加载完毕
      setInitializing(false);
    }
  }, [currentData, cohortsData, setCurrentCohort]);

  const handleCohortChange = async (cohortId: number) => {
    try {
      await cohortService.setCurrent(cohortId);
      const cohort = cohorts.find((c) => c.id === cohortId);
      if (cohort) {
        setCurrentCohort(cohort);
      }
      message.success(`已切换到届次`);
      // 清除所有依赖于届次的查询缓存，触发页面自然重渲染
      queryClient.invalidateQueries();
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

  const handleThemeChange: MenuProps['onClick'] = ({ key }) => {
    setThemeMode(key as ThemeMode);
  };

  // 根据主题模式计算动态颜色
  const borderColor = isDark ? '#303030' : '#f0f0f0';
  const headerBg = isDark ? '#141414' : '#fff';

  return (
    <Layout style={{ height: '100vh', overflow: 'hidden' }}>
      <Sider
        trigger={null}
        collapsible
        collapsed={sidebarCollapsed}
        theme={isDark ? 'dark' : 'light'}
        style={{
          borderRight: `1px solid ${borderColor}`,
          overflow: 'hidden',
          height: '100vh',
        }}
      >
        <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
          {/* 标题区域 */}
          <div
            style={{
              height: 64,
              flexShrink: 0,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderBottom: `1px solid ${borderColor}`,
            }}
          >
            <Typography.Title level={5} style={{ margin: 0, whiteSpace: 'nowrap' }}>
              {sidebarCollapsed ? '班管' : '班级事务管理系统'}
            </Typography.Title>
          </div>

          {/* 导航菜单 — 占满剩余空间 */}
          <div style={{ flex: 1, overflow: 'auto', minHeight: 0 }}>
            <Menu
              mode="inline"
              selectedKeys={[location.pathname]}
              items={menuItems}
              onClick={handleMenuClick}
            />
          </div>

          {/* 主题切换 — 固定在侧边栏底部 */}
          <div
            style={{
              flexShrink: 0,
              borderTop: `1px solid ${borderColor}`,
              padding: sidebarCollapsed ? '8px 4px' : '8px 4px',
            }}
          >
            <Dropdown
              menu={{
                items: themeMenuItems,
                selectable: true,
                selectedKeys: [themeMode],
                onClick: handleThemeChange,
              }}
              trigger={['hover']}
              placement="topRight"
            >
              <Button
                type="text"
                icon={themeIconMap[themeMode]}
                block
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: sidebarCollapsed ? 'center' : 'flex-start',
                  padding: sidebarCollapsed ? '4px 0' : '4px 16px',
                  height: 36,
                }}
              >
                {!sidebarCollapsed && (
                  <span style={{ marginLeft: 8 }}>
                    {themeMode === 'light' ? '浅色模式' : themeMode === 'dark' ? '深色模式' : '跟随系统'}
                  </span>
                )}
              </Button>
            </Dropdown>
          </div>
        </div>
      </Sider>
      <Layout style={{ minWidth: 0, height: '100vh', overflow: 'hidden' }}>
        <Header
          style={{
            background: headerBg,
            padding: '0 24px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            borderBottom: `1px solid ${borderColor}`,
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

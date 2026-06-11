import { lazy, Suspense } from 'react';
import { createBrowserRouter, Navigate } from 'react-router-dom';
import { Spin } from 'antd';
import AppLayout from '@/layouts/AppLayout';

const Dashboard = lazy(() => import('@/features/dashboard/Dashboard'));
const CohortList = lazy(() => import('@/features/cohorts/CohortList'));
const StudentList = lazy(() => import('@/features/students/StudentList'));
const StudentDetail = lazy(() => import('@/features/students/StudentDetail'));
const HomeworkList = lazy(() => import('@/features/homework/HomeworkList'));
const HomeworkDetail = lazy(() => import('@/features/homework/HomeworkDetail'));
const AttendancePage = lazy(() => import('@/features/attendance/AttendancePage'));
const ScoreManagement = lazy(() => import('@/features/scores/ScoreManagement'));
const AffairsPage = lazy(() => import('@/features/affairs/AffairsPage'));
const StatisticsPage = lazy(() => import('@/features/statistics/StatisticsPage'));
const SettingsPage = lazy(() => import('@/features/settings/SettingsPage'));

// eslint-disable-next-line react-refresh/only-export-components
function LazyLoad({ children }: { children: React.ReactNode }) {
  return (
    <Suspense
      fallback={
        <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '300px' }}>
          <Spin size="large" />
        </div>
      }
    >
      {children}
    </Suspense>
  );
}

export const router = createBrowserRouter([
  {
    path: '/',
    element: <AppLayout />,
    children: [
      { index: true, element: <Navigate to="/dashboard" replace /> },
      {
        path: 'dashboard',
        element: (
          <LazyLoad>
            <Dashboard />
          </LazyLoad>
        ),
      },
      {
        path: 'cohorts',
        element: (
          <LazyLoad>
            <CohortList />
          </LazyLoad>
        ),
      },
      {
        path: 'students',
        element: (
          <LazyLoad>
            <StudentList />
          </LazyLoad>
        ),
      },
      {
        path: 'students/:id',
        element: (
          <LazyLoad>
            <StudentDetail />
          </LazyLoad>
        ),
      },
      {
        path: 'homework',
        element: (
          <LazyLoad>
            <HomeworkList />
          </LazyLoad>
        ),
      },
      {
        path: 'homework/:id',
        element: (
          <LazyLoad>
            <HomeworkDetail />
          </LazyLoad>
        ),
      },
      {
        path: 'attendance',
        element: (
          <LazyLoad>
            <AttendancePage />
          </LazyLoad>
        ),
      },
      {
        path: 'scores',
        element: (
          <LazyLoad>
            <ScoreManagement />
          </LazyLoad>
        ),
      },
      {
        path: 'affairs',
        element: (
          <LazyLoad>
            <AffairsPage />
          </LazyLoad>
        ),
      },
      {
        path: 'statistics',
        element: (
          <LazyLoad>
            <StatisticsPage />
          </LazyLoad>
        ),
      },
      {
        path: 'settings',
        element: (
          <LazyLoad>
            <SettingsPage />
          </LazyLoad>
        ),
      },
    ],
  },
]);

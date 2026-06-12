import { Component, type ReactNode } from 'react';
import { Button, Result } from 'antd';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error, errorInfo);
  }

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100vh',
            background: '#f5f5f5',
          }}
        >
          <Result
            status="error"
            title="页面发生异常"
            subTitle={this.state.error?.message || '未知错误，请尝试刷新页面'}
            extra={
              <Button type="primary" onClick={this.handleReload}>
                刷新页面
              </Button>
            }
          />
        </div>
      );
    }
    return this.props.children;
  }
}

import React, { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RefreshCw, Trash2, Home } from 'lucide-react';

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
  onReset?: () => void;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null
  };

  public static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Bapu Studio Caught Unhandled React Error:', error, errorInfo);
    this.setState({ errorInfo });
  }

  private handleReload = () => {
    this.setState({ hasError: false, error: null, errorInfo: null });
    if (this.props.onReset) {
      this.props.onReset();
    }
  };

  private handleResetStorage = () => {
    try {
      localStorage.removeItem('bapu_databases');
    } catch {}
    window.location.reload();
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100%',
          minHeight: '400px',
          padding: '32px',
          background: 'var(--bg-card, #0f172a)',
          color: 'var(--text-main, #f1f5f9)',
          textAlign: 'center'
        }}>
          <div style={{
            width: '48px',
            height: '48px',
            borderRadius: '50%',
            background: 'rgba(239, 68, 68, 0.15)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: '16px',
            border: '1px solid rgba(239, 68, 68, 0.3)'
          }}>
            <AlertTriangle size={24} color="#f87171" />
          </div>

          <h2 style={{ fontSize: '18px', fontWeight: 700, marginBottom: '8px', color: 'var(--text-main, #f1f5f9)' }}>
            {this.props.fallbackTitle || 'Something went wrong in this view'}
          </h2>

          <p style={{ fontSize: '12px', color: 'var(--text-dim, #94a3b8)', maxWidth: '500px', marginBottom: '20px', lineHeight: 1.5 }}>
            {this.state.error?.message || 'An unexpected error occurred while rendering the workspace.'}
          </p>

          <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap', justifyContent: 'center' }}>
            <button
              onClick={this.handleReload}
              className="btn-send"
              style={{ padding: '8px 16px', display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px' }}
            >
              <RefreshCw size={14} />
              <span>Retry View</span>
            </button>

            <button
              onClick={this.handleResetStorage}
              className="btn-secondary"
              style={{ padding: '8px 16px', display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: '#f87171', borderColor: 'rgba(239, 68, 68, 0.3)' }}
            >
              <Trash2 size={14} />
              <span>Reset Database Storage</span>
            </button>
          </div>

          {this.state.error && (
            <details style={{ marginTop: '24px', textAlign: 'left', maxWidth: '600px', width: '100%' }}>
              <summary style={{ fontSize: '11px', color: 'var(--text-dim, #64748b)', cursor: 'pointer', outline: 'none' }}>
                View diagnostic stack trace
              </summary>
              <pre style={{
                marginTop: '8px',
                padding: '12px',
                background: '#020617',
                border: '1px solid var(--border-subtle, #1e293b)',
                borderRadius: '6px',
                fontSize: '10px',
                color: '#f87171',
                overflowX: 'auto',
                fontFamily: 'var(--font-mono, monospace)',
                lineHeight: 1.4
              }}>
                {this.state.error.stack || this.state.error.message}
              </pre>
            </details>
          )}
        </div>
      );
    }

    return this.props.children;
  }
}

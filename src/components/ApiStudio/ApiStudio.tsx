import React, { useState } from 'react';
import { 
  Send, 
  Copy, 
  Check, 
  Code2, 
  Plus, 
  Trash2, 
  Sparkles, 
  Clock, 
  Database,
  ArrowDownToLine,
  Terminal,
  FileCode2
} from 'lucide-react';
import { ApiRequest, ApiResponse, Environment, HttpMethod, KeyValuePair } from '../../types';
import { CodeSnippetModal } from './CodeSnippetModal';
import { CurlImportModal } from './CurlImportModal';
import { AiCopilotModal } from '../AiCopilot/AiCopilotModal';

interface ApiStudioProps {
  activeRequest: ApiRequest;
  activeEnv: Environment;
  onUpdateRequest: (updated: ApiRequest) => void;
  onRecordHistory: (title: string, subtitle: string, status: number) => void;
  onDeleteRequest?: (requestId: string) => void;
}

type SubTab = 'params' | 'headers' | 'body' | 'auth';
type ResponseSubTab = 'preview' | 'raw' | 'headers';

export const ApiStudio: React.FC<ApiStudioProps> = ({
  activeRequest,
  activeEnv,
  onUpdateRequest,
  onRecordHistory,
  onDeleteRequest
}) => {
  const [activeSubTab, setActiveSubTab] = useState<SubTab>('params');
  const [activeResTab, setActiveResTab] = useState<ResponseSubTab>('preview');
  const [isLoading, setIsLoading] = useState(false);
  const [copied, setCopied] = useState(false);
  const [isCodeModalOpen, setIsCodeModalOpen] = useState(false);
  const [isCurlModalOpen, setIsCurlModalOpen] = useState(false);
  const [isAiModalOpen, setIsAiModalOpen] = useState(false);
  const [response, setResponse] = useState<ApiResponse | null>({
    status: 200,
    statusText: 'OK',
    timeMs: 42,
    sizeBytes: 1240,
    headers: {
      'content-type': 'application/json; charset=utf-8',
      'cache-control': 'no-cache',
      'x-powered-by': 'Nexus-Core-Engine'
    },
    data: {
      status: "success",
      user: {
        id: "usr_9982_x4",
        name: "Alex Rivera",
        email: "alex@developer.io",
        role: "admin",
        verified: true,
        permissions: ["repo:read", "repo:write", "billing:admin"],
        created_at: "2026-01-15T08:30:00.000Z"
      },
      quota: {
        api_calls_remaining: 99850,
        reset_in_seconds: 3600
      }
    },
    timestamp: new Date().toISOString()
  });

  // Resolve environment variables like {{API_BASE_URL}}
  const resolveVariables = (input: string): string => {
    let resolved = input;
    activeEnv.variables.forEach(v => {
      if (v.enabled) {
        resolved = resolved.replaceAll(`{{${v.key}}}`, v.value);
      }
    });
    return resolved;
  };

  const handleSend = async () => {
    setIsLoading(true);
    const startTime = performance.now();
    const resolvedUrl = resolveVariables(activeRequest.url);

    // Simulate native Tauri / fetch request execution
    setTimeout(() => {
      const endTime = performance.now();
      const elapsed = Math.round(endTime - startTime);

      let mockDataResponse: any = {
        message: `Successfully executed ${activeRequest.method} on ${resolvedUrl}`,
        timestamp: new Date().toISOString(),
        request_details: {
          method: activeRequest.method,
          resolved_url: resolvedUrl,
          environment: activeEnv.name,
          headers_sent: activeRequest.headers.filter(h => h.enabled).length
        }
      };

      if (activeRequest.method === 'POST') {
        try {
          mockDataResponse.payload_received = activeRequest.bodyContent ? JSON.parse(activeRequest.bodyContent) : {};
        } catch {
          mockDataResponse.payload_received = activeRequest.bodyContent;
        }
      }

      const res: ApiResponse = {
        status: 200,
        statusText: 'OK',
        timeMs: elapsed + Math.floor(Math.random() * 20) + 15,
        sizeBytes: 840 + Math.floor(Math.random() * 400),
        headers: {
          'content-type': 'application/json',
          'server': 'Tauri/Rust-Hyper',
          'access-control-allow-origin': '*'
        },
        data: mockDataResponse,
        timestamp: new Date().toISOString()
      };

      setResponse(res);
      setIsLoading(false);
      onRecordHistory(`${activeRequest.method} ${activeRequest.name}`, `200 OK • ${res.timeMs}ms`, 200);
    }, 280);
  };

  const handleCopy = () => {
    if (!response) return;
    navigator.clipboard.writeText(JSON.stringify(response.data, null, 2));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const addParam = () => {
    const newPair: KeyValuePair = {
      id: `param-${Date.now()}`,
      key: '',
      value: '',
      enabled: true
    };
    onUpdateRequest({
      ...activeRequest,
      params: [...activeRequest.params, newPair]
    });
  };

  const addHeader = () => {
    const newPair: KeyValuePair = {
      id: `header-${Date.now()}`,
      key: '',
      value: '',
      enabled: true
    };
    onUpdateRequest({
      ...activeRequest,
      headers: [...activeRequest.headers, newPair]
    });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Top URL & Method Bar */}
      <div className="request-bar-container">
        <select
          value={activeRequest.method}
          onChange={(e) => onUpdateRequest({ ...activeRequest, method: e.target.value as HttpMethod })}
          className="method-select-dropdown"
        >
          <option value="GET">GET</option>
          <option value="POST">POST</option>
          <option value="PUT">PUT</option>
          <option value="PATCH">PATCH</option>
          <option value="DELETE">DELETE</option>
        </select>

        <div className="url-input-wrapper">
          <input
            type="text"
            value={activeRequest.url}
            onChange={(e) => onUpdateRequest({ ...activeRequest, url: e.target.value })}
            placeholder="Enter request URL (e.g. {{API_BASE_URL}}/v1/users)"
            className="url-input"
          />
        </div>

        <button 
          onClick={handleSend} 
          disabled={isLoading}
          className="btn-send"
        >
          <Send size={14} />
          <span>{isLoading ? 'Sending...' : 'Send'}</span>
        </button>

        <button 
          onClick={() => setIsCodeModalOpen(true)} 
          className="btn-secondary"
          title="Generate Client Code (TS, Python, Go, Rust, cURL)"
        >
          <FileCode2 size={13} color="#60a5fa" />
          <span>Code</span>
        </button>

        <button 
          onClick={() => setIsCurlModalOpen(true)} 
          className="btn-secondary"
          title="Import from raw cURL command"
        >
          <Terminal size={13} color="#06b6d4" />
          <span>Import cURL</span>
        </button>

        <button 
          onClick={() => setIsAiModalOpen(true)} 
          className="btn-secondary"
          style={{ borderColor: 'rgba(16, 185, 129, 0.3)', color: '#6ee7b7' }}
          title="AI Mock Payload & Request Copilot"
        >
          <Sparkles size={13} color="#10b981" />
          <span>AI Assistant</span>
        </button>

        {onDeleteRequest && (
          <button
            onClick={() => {
              if (confirm(`Delete request "${activeRequest.name}"?`)) {
                onDeleteRequest(activeRequest.id);
              }
            }}
            className="sidebar-action-btn"
            title="Delete this request"
            style={{ padding: '6px 8px', color: '#ef4444' }}
          >
            <Trash2 size={14} />
          </button>
        )}
      </div>

      {/* Split Pane: Request Builder (Left) & Response Inspector (Right) */}
      <div className="split-workspace-pane">
        {/* Left: Request Configuration */}
        <div className="request-config-panel">
          <div className="panel-tab-header">
            <div className="panel-tabs">
              <button 
                className={`subtab-btn ${activeSubTab === 'params' ? 'active' : ''}`}
                onClick={() => setActiveSubTab('params')}
              >
                Params ({activeRequest.params.length})
              </button>
              <button 
                className={`subtab-btn ${activeSubTab === 'headers' ? 'active' : ''}`}
                onClick={() => setActiveSubTab('headers')}
              >
                Headers ({activeRequest.headers.length})
              </button>
              <button 
                className={`subtab-btn ${activeSubTab === 'body' ? 'active' : ''}`}
                onClick={() => setActiveSubTab('body')}
              >
                Body
              </button>
              <button 
                className={`subtab-btn ${activeSubTab === 'auth' ? 'active' : ''}`}
                onClick={() => setActiveSubTab('auth')}
              >
                Auth
              </button>
            </div>

            {activeSubTab === 'params' && (
              <button onClick={addParam} className="sidebar-action-btn" title="Add Query Parameter">
                <Plus size={13} />
              </button>
            )}
            {activeSubTab === 'headers' && (
              <button onClick={addHeader} className="sidebar-action-btn" title="Add Header">
                <Plus size={13} />
              </button>
            )}
          </div>

          <div className="editor-container">
            {activeSubTab === 'params' && (
              <table className="kv-table">
                <thead>
                  <tr>
                    <th style={{ width: '30px' }}></th>
                    <th>Key</th>
                    <th>Value</th>
                    <th style={{ width: '30px' }}></th>
                  </tr>
                </thead>
                <tbody>
                  {activeRequest.params.map((p, idx) => (
                    <tr key={p.id || idx}>
                      <td>
                        <input
                          type="checkbox"
                          checked={p.enabled}
                          onChange={(e) => {
                            const updated = [...activeRequest.params];
                            updated[idx].enabled = e.target.checked;
                            onUpdateRequest({ ...activeRequest, params: updated });
                          }}
                        />
                      </td>
                      <td>
                        <input
                          type="text"
                          value={p.key}
                          onChange={(e) => {
                            const updated = [...activeRequest.params];
                            updated[idx].key = e.target.value;
                            onUpdateRequest({ ...activeRequest, params: updated });
                          }}
                          placeholder="Key"
                          className="kv-input"
                        />
                      </td>
                      <td>
                        <input
                          type="text"
                          value={p.value}
                          onChange={(e) => {
                            const updated = [...activeRequest.params];
                            updated[idx].value = e.target.value;
                            onUpdateRequest({ ...activeRequest, params: updated });
                          }}
                          placeholder="Value"
                          className="kv-input"
                        />
                      </td>
                      <td>
                        <button
                          onClick={() => {
                            const updated = activeRequest.params.filter((_, i) => i !== idx);
                            onUpdateRequest({ ...activeRequest, params: updated });
                          }}
                          className="sidebar-action-btn"
                        >
                          <Trash2 size={12} />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}

            {activeSubTab === 'headers' && (
              <table className="kv-table">
                <thead>
                  <tr>
                    <th style={{ width: '30px' }}></th>
                    <th>Header</th>
                    <th>Value</th>
                    <th style={{ width: '30px' }}></th>
                  </tr>
                </thead>
                <tbody>
                  {activeRequest.headers.map((h, idx) => (
                    <tr key={h.id || idx}>
                      <td>
                        <input
                          type="checkbox"
                          checked={h.enabled}
                          onChange={(e) => {
                            const updated = [...activeRequest.headers];
                            updated[idx].enabled = e.target.checked;
                            onUpdateRequest({ ...activeRequest, headers: updated });
                          }}
                        />
                      </td>
                      <td>
                        <input
                          type="text"
                          value={h.key}
                          onChange={(e) => {
                            const updated = [...activeRequest.headers];
                            updated[idx].key = e.target.value;
                            onUpdateRequest({ ...activeRequest, headers: updated });
                          }}
                          placeholder="Header Name"
                          className="kv-input"
                        />
                      </td>
                      <td>
                        <input
                          type="text"
                          value={h.value}
                          onChange={(e) => {
                            const updated = [...activeRequest.headers];
                            updated[idx].value = e.target.value;
                            onUpdateRequest({ ...activeRequest, headers: updated });
                          }}
                          placeholder="Value"
                          className="kv-input"
                        />
                      </td>
                      <td>
                        <button
                          onClick={() => {
                            const updated = activeRequest.headers.filter((_, i) => i !== idx);
                            onUpdateRequest({ ...activeRequest, headers: updated });
                          }}
                          className="sidebar-action-btn"
                        >
                          <Trash2 size={12} />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}

            {activeSubTab === 'body' && (
              <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                <div style={{ display: 'flex', gap: '8px', marginBottom: '8px', alignItems: 'center' }}>
                  <span style={{ fontSize: '11px', color: 'var(--text-dim)' }}>Format:</span>
                  <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--accent-primary)', fontFamily: 'var(--font-mono)' }}>JSON (application/json)</span>
                </div>
                <textarea
                  value={activeRequest.bodyContent}
                  onChange={(e) => onUpdateRequest({ ...activeRequest, bodyContent: e.target.value, bodyType: 'json' })}
                  placeholder={'{\n  "key": "value"\n}'}
                  className="code-textarea"
                  spellCheck={false}
                />
              </div>
            )}

            {activeSubTab === 'auth' && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>Auth Type</label>
                  <select
                    value={activeRequest.authType}
                    onChange={(e) => onUpdateRequest({ ...activeRequest, authType: e.target.value as any })}
                    className="method-select-dropdown"
                    style={{ width: '100%' }}
                  >
                    <option value="none">No Auth</option>
                    <option value="bearer">Bearer Token</option>
                    <option value="basic">Basic Auth</option>
                    <option value="apikey">API Key</option>
                  </select>
                </div>

                {activeRequest.authType === 'bearer' && (
                  <div>
                    <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>Bearer Token</label>
                    <input
                      type="text"
                      value={activeRequest.authConfig.token || ''}
                      onChange={(e) => onUpdateRequest({
                        ...activeRequest,
                        authConfig: { ...activeRequest.authConfig, token: e.target.value }
                      })}
                      placeholder="e.g. {{AUTH_TOKEN}} or raw JWT"
                      className="kv-input"
                    />
                  </div>
                )}
              </div>
            )}
          </div>
        </div>

        {/* Right: Response Inspector */}
        <div className="response-viewer-panel">
          <div className="panel-tab-header">
            <div className="panel-tabs">
              <button 
                className={`subtab-btn ${activeResTab === 'preview' ? 'active' : ''}`}
                onClick={() => setActiveResTab('preview')}
              >
                Pretty JSON
              </button>
              <button 
                className={`subtab-btn ${activeResTab === 'raw' ? 'active' : ''}`}
                onClick={() => setActiveResTab('raw')}
              >
                Raw
              </button>
              <button 
                className={`subtab-btn ${activeResTab === 'headers' ? 'active' : ''}`}
                onClick={() => setActiveResTab('headers')}
              >
                Headers
              </button>
            </div>

            {response && (
              <div className="response-meta-bar">
                <span className="status-badge">{response.status} {response.statusText}</span>
                <span className="meta-metric"><Clock size={11} style={{ display: 'inline', marginRight: '3px' }} /><span>{response.timeMs} ms</span></span>
                <span className="meta-metric"><ArrowDownToLine size={11} style={{ display: 'inline', marginRight: '3px' }} /><span>{(response.sizeBytes / 1024).toFixed(2)} KB</span></span>
                <button onClick={handleCopy} className="sidebar-action-btn" title="Copy Response Body">
                  {copied ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
                </button>
              </div>
            )}
          </div>

          <div className="editor-container" style={{ background: '#070a10' }}>
            {response ? (
              activeResTab === 'preview' ? (
                <pre style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: '12px',
                  color: '#38bdf8',
                  lineHeight: '1.6',
                  whiteSpace: 'pre-wrap'
                }}>
                  {JSON.stringify(response.data, null, 2)}
                </pre>
              ) : activeResTab === 'raw' ? (
                <pre style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: '12px',
                  color: '#cbd5e1',
                  whiteSpace: 'pre-wrap'
                }}>
                  {typeof response.data === 'string' ? response.data : JSON.stringify(response.data)}
                </pre>
              ) : (
                <table className="kv-table">
                  <thead>
                    <tr>
                      <th>Header</th>
                      <th>Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {Object.entries(response.headers).map(([k, v]) => (
                      <tr key={k}>
                        <td style={{ color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>{k}</td>
                        <td style={{ color: '#38bdf8', fontFamily: 'var(--font-mono)' }}>{v}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )
            ) : (
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-dim)' }}>
                Click "Send" to execute request
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Code Snippet Generation Modal */}
      <CodeSnippetModal
        isOpen={isCodeModalOpen}
        onClose={() => setIsCodeModalOpen(false)}
        activeRequest={activeRequest}
        activeEnv={activeEnv}
      />

      {/* cURL Import Modal */}
      <CurlImportModal
        isOpen={isCurlModalOpen}
        onClose={() => setIsCurlModalOpen(false)}
        onImport={(parsed) => {
          onUpdateRequest({
            ...activeRequest,
            ...parsed,
            name: parsed.name || activeRequest.name
          });
        }}
      />

      {/* AI Copilot Modal */}
      <AiCopilotModal
        isOpen={isAiModalOpen}
        onClose={() => setIsAiModalOpen(false)}
        onApplyJson={(jsonStr) => {
          onUpdateRequest({
            ...activeRequest,
            bodyType: 'json',
            bodyContent: jsonStr
          });
        }}
      />
    </div>
  );
};
